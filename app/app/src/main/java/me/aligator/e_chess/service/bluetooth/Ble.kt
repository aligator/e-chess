package me.aligator.e_chess.service.bluetooth

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.util.Log
import androidx.annotation.RequiresPermission
import androidx.core.content.ContextCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import java.security.Permission
import java.util.UUID

data class SimpleDevice(
    val device: BluetoothDevice,
    val address: String,
    val name: String?,
)

enum class ConnectionStep {
    /**
     * Bluetooth is not available.
     */
    UNAVAILABLE,

    /**
     * Bluetooth is currently not enabled.
     */
    DISABLED,

    /**
     * Check required permissions.
     */
    PERMISSIONS,

    /**
     * All is set to start using ble but it is currently idle.
     */
    IDLE,

    /**
     * Devices are scanned.
     */
    SCANNING,
}

enum class DeviceState {
    CONNECTED,
    CONNECTING,
    DISCONNECTING,
    DISCONNECTED,
    UNKNOWN
}

data class ConnectedDevice(
    val deviceState: DeviceState,
    val address: String?,
)

data class BleState(
    /**
     * The current connection step in which ble is in.
     */
    val step: ConnectionStep = ConnectionStep.UNAVAILABLE,

    val connectedDevice: ConnectedDevice = ConnectedDevice(
        deviceState = DeviceState.UNKNOWN,
        address = null,
    ),

    /**
     * List of found devices, filtered by the specific service id of the chess board.
     */
    val devices: List<SimpleDevice> = emptyList(),
)

private const val LOG_TAG = "BLE"

data class BleResponse(
    val characteristic: UUID,
    val data: ByteArray,
    /**
     * BluetoothGattCharacteristic.WRITE_TYPE_*
     */
    val writeType: Int = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as BleResponse

        if (characteristic != other.characteristic) return false
        if (!data.contentEquals(other.data)) return false

        return true
    }

    override fun hashCode(): Int {
        var result = characteristic.hashCode()
        result = 31 * result + data.contentHashCode()
        return result
    }
};


fun requiredPermissions(): List<String>{
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        return listOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT)
    } else {
        return listOf(Manifest.permission.ACCESS_FINE_LOCATION)
    }
}

fun hasPermissions(context: Context): Boolean {
    val permissions = requiredPermissions()

    return permissions.all { permission ->
        ContextCompat.checkSelfPermission(context, permission) ==
                PackageManager.PERMISSION_GRANTED
    }
}


/**
 * Wrapper for the android ble functionality that
 * * handles permission setup
 * * connection procedure
 * * abstracts away differences between android version
 * * provides utilities for sending and receiving larger data (chunking)
 */
class Ble(
    private val parentScope: CoroutineScope,
    val context: Context,
    val serviceUuid: UUID
) {
    private val maxChunkSize = 20

    /// current connection state
    val _bleState = MutableStateFlow(BleState())
    val bleState: StateFlow<BleState> = _bleState.asStateFlow()

    /// bluetooth device handles
    private val bluetoothManager by lazy {
        context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
    }
    private val adapter: BluetoothAdapter?
        get() = bluetoothManager.adapter
    private val scanner: BluetoothLeScanner?
        get() = adapter?.bluetoothLeScanner

    private val mainLoopHandler = Handler(Looper.getMainLooper())

    private var currentScanCallback: ScanCallback? = null

    private var gatt: BluetoothGatt? = null

    private var responseJob: Job? = null
    private var responseAckJob: Job? = null
    private var responseChannel: Channel<BleResponse> = Channel()
    // There should always be only one pending response at a time
    private var responseAckChannel: Channel<UUID> = Channel(1)

    private fun setDeviceState(
        state: DeviceState,
        address: String?
    ) {
        // The device events may be running on another thread.
        // So this must use the mainLoopHandler.
        mainLoopHandler.post {
            _bleState.update {
                it.copy(connectedDevice = it.connectedDevice.copy(
                    deviceState = state,
                    address= address,
                ))
            }
        }
    }

    private fun setStep(newStep: ConnectionStep) {
        // update the state
        _bleState.update {
            it.copy(step = newStep)
        }
    }

    /**
     * Checks for all required things to use ble.
     * It resets the current step to the respective step if needed.
     * If all is fine, it does not change the step.
     */
    fun checkBluetooth(): Boolean {
        if (adapter == null || scanner == null) {
            setStep(ConnectionStep.UNAVAILABLE)
        } else if (adapter?.isEnabled == true) {
            // If the permissions are already correct
            // skip the permission check.
            if (!hasPermissions(context)) {
                setStep(ConnectionStep.PERMISSIONS)
                return false
            }
        } else if (adapter?.isEnabled == false) {
            setStep(ConnectionStep.DISABLED)
            return false
        }

        if (bleState.value.step == ConnectionStep.UNAVAILABLE) {
            setStep(ConnectionStep.IDLE)
        }

        return true
    }

    fun onDestroy() {
        stopScan()
    }

    fun startScan() {
        if (!checkBluetooth()) {
            return
        }

        // If already scanning - do nothing
        if (currentScanCallback != null) {
            return
        }

        val callback =
            object : ScanCallback() {
                override fun onScanResult(callbackType: Int, result: ScanResult?) {
                    val address = result?.device?.address ?: return
                    // BLE scan callbacks happen off the main thread; push updates to UI state
                    // onto the main looper.
                    mainLoopHandler.post {
                        _bleState.update  { state ->

                            val name = try {
                                result.device.name
                            } catch (ex: SecurityException) {
                                Log.e(LOG_TAG, "could not get the device name $ex")
                                null
                            }

                            val updated = state.devices.toMutableList()
                            val existingIndex = updated.indexOfFirst { it.address == address }

                            val newDevice = SimpleDevice(result.device, address, name)
                            if (existingIndex >= 0) {
                                updated[existingIndex] = newDevice
                            } else {
                                updated.add(newDevice)
                            }
                            state.copy(devices = updated)
                        }
                    }
                }

                override fun onScanFailed(errorCode: Int) {
                    Log.e(LOG_TAG, "Scan failed $errorCode")
                    setStep(ConnectionStep.IDLE)
                }
            }

        // First clear the list of devices.
        _bleState.update {
            it.copy(devices = emptyList())
        }

        val settings =
            ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
        val filter: List<ScanFilter> = listOf(
            ScanFilter.Builder().setServiceUuid(ParcelUuid(serviceUuid)).build()
        )

        try {
            scanner?.startScan(null, settings, callback)
            currentScanCallback = callback
            setStep(ConnectionStep.SCANNING)
        } catch (se: SecurityException) {
            Log.e(LOG_TAG, "startScan without permission", se)
            setStep(ConnectionStep.PERMISSIONS)
        }
    }

    fun stopScan() {
        if (currentScanCallback == null) {
            return
        }

        try {
            scanner?.stopScan(currentScanCallback)
            setStep(ConnectionStep.IDLE)
        } catch (se: SecurityException) {
            Log.e(LOG_TAG, "stopScan without permission", se)
            setStep(ConnectionStep.PERMISSIONS)
        } finally {
            currentScanCallback = null
        }
    }

    fun handleServiceDiscovered(gatt: BluetoothGatt) {
        // TODO: add listener and call all that need to know


        // TODO: pass event to the parent classes so that these can handle their
        // respective characteristics.
//
//                txCharacteristic = service.getCharacteristic(DATA_TX_CHARACTERISTIC_UUID)
//                rxCharacteristic = service.getCharacteristic(DATA_RX_CHARACTERISTIC_UUID)
//                gameLoadCharacteristic = service.getCharacteristic(GAME_KEY_CHARACTERISTIC_UUID)
//                if (txCharacteristic == null ||
//                    rxCharacteristic == null ||
//                    gameLoadCharacteristic == null
//                ) {
//                    Log.e(LOG_TAG, "Charakteristiken nicht gefunden")
//                    postState(onStateChange, "Charakteristik fehlt", false)
//                    return
//                }
//
//                enableNotifications(gatt, txCharacteristic!!)
//                postState(onStateChange, "Verbunden und bereit", true)
//                Log.d(LOG_TAG, "connected to ble")
    }

    fun handleCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        // TODO: add listener and call all that need to know
    }

    fun handleCharacteristicWrite(
        gatt: BluetoothGatt?,
        characteristic: BluetoothGattCharacteristic?,
        status: Int
    ) {
        if (characteristic?.writeType == BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT && characteristic.uuid != null && status ==  BluetoothGatt.GATT_SUCCESS) {
            parentScope.launch {
                responseAckChannel.send(characteristic.uuid)
            }
        }
    }

    private fun createCallback() =
        object : BluetoothGattCallback() {

            @RequiresPermission(Manifest.permission.BLUETOOTH_CONNECT)
            override fun onConnectionStateChange(
                gatt: BluetoothGatt,
                status: Int,
                newState: Int
            ) {
                val state =
                    when (newState) {
                        BluetoothProfile.STATE_CONNECTED -> DeviceState.CONNECTED
                        BluetoothProfile.STATE_CONNECTING -> DeviceState.CONNECTING
                        BluetoothProfile.STATE_DISCONNECTING -> DeviceState.DISCONNECTING
                        BluetoothProfile.STATE_DISCONNECTED -> DeviceState.DISCONNECTED
                        else -> DeviceState.UNKNOWN
                    }
                setDeviceState(state, gatt.device.address)
                if (newState == BluetoothProfile.STATE_CONNECTED) {
                    gatt.discoverServices()
                }
                if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                    close()
                }
            }

            override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                val service = gatt.getService(serviceUuid)
                if (service == null) {
                    // Not the service we need.
                    return
                }

                handleServiceDiscovered(gatt)
            }

            override fun onCharacteristicChanged(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                value: ByteArray
            ) {
                Log.d(LOG_TAG, "received characteristic change: $value")
                handleCharacteristicChanged(gatt, characteristic, value)
            }

            override fun onCharacteristicWrite(
                gatt: BluetoothGatt?,
                characteristic: BluetoothGattCharacteristic?,
                status: Int
            ) {
                Log.d(LOG_TAG, "characteristic ${characteristic?.uuid} written: $status")
                handleCharacteristicWrite(gatt, characteristic, status)
            }
        }

    private suspend fun responseLoop() {
        responseAckChannel.close()
        responseAckChannel = Channel(1)

        for (response in responseChannel) {
            val currentGatt = gatt ?: break
            val service = currentGatt.getService(serviceUuid)
            val characteristic = service.getCharacteristic(response.characteristic)

            // only allow chunking for WRITE_TYPE_DEFAULT
            val chunks: List<ByteArray> = if (response.writeType == BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) {
                response.data.asList().chunked(this.maxChunkSize).map { it.toByteArray() }.toList()
            } else {
                listOf(response.data)
            }

            for (chunk in chunks) {
                Log.d(LOG_TAG, "send chunk of message ${chunk.decodeToString()}")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    currentGatt.writeCharacteristic(
                        characteristic,
                        chunk,
                        response.writeType
                    )
                } else {
                    @Suppress("DEPRECATION")
                    val ok = currentGatt.writeCharacteristic(
                        characteristic.apply {
                            value = chunk
                            response.writeType
                        }
                    )
                    if (!ok) {
                        Log.e(LOG_TAG, "writeCharacteristic fehlgeschlagen")
                    }
                }

                if (response.writeType == BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) {
                    // If WRITE_TYPE_DEFAULT - wait for the ack in  onCharacteristicWrite
                    val ack = withTimeout(10000L) {
                        responseAckChannel.receive()
                    }

                    if (ack != response) {
                        Log.e(
                            LOG_TAG,
                            "Something went wrong when waiting for the ack. It is not the expected response."
                        )
                        return
                    }
                }
            }
        }
    }


    fun connect(device: BluetoothDevice) {
        if (!checkBluetooth()) {
            return
        }

        close()

        @SuppressLint("MissingPermission") // checked in checkBluetooth already
        gatt = device.connectGatt(context, false, createCallback())

        // Start background thread to send the queued responses.
        responseJob = parentScope.launch {
            responseLoop()
        }
    }

    fun close() {
        //TODO: implement - maybe with the listeners of the parent classes?

        responseJob?.cancel()
        responseJob = null


//        pendingBuffer.clear()
//        rxCharacteristic = null
//        txCharacteristic = null
//        gameLoadCharacteristic = null
        gatt?.close()
        gatt = null
    }

    private fun sendCharacteristic(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        payload: ByteArray
    ) {
        parentScope.launch {
            responseChannel.send(BleResponse(
                characteristic = characteristic.uuid,
                data = payload
            ))

            Log.d(LOG_TAG, "enqueued message to ${characteristic.uuid}: ${payload.decodeToString()}")
        }
    }
}