package me.aligator.e_chess.service

import android.Manifest
import android.annotation.SuppressLint
import android.app.Service
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Binder
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.ParcelUuid
import android.util.Log
import android.widget.Toast
import androidx.core.content.ContextCompat
import java.io.BufferedInputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.nio.charset.StandardCharsets
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject
import java.util.Scanner

data class SimpleDevice(
    val device: BluetoothDevice,
    val address: String,
    val name: String?,
)

data class BleUiState(
    val scanning: Boolean = false,
    val connectionState: String = "Keine Verbindung",
    val devices: List<SimpleDevice> = emptyList(),
    val canLoadGame: Boolean = false,
)

private const val PROTOCOL_VERSION = 1
private val SERVICE_UUID: UUID = UUID.fromString("b4d75b6c-7284-4268-8621-6e3cef3c6ac4")

/**
 * Characteristic for bridging http requests via ble.file This Characteristic is for "BoardToPhone"
 * direction (e.g. Transmit from the board to android).
 */
private val DATA_TX_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("aa8381af-049a-46c2-9c92-1db7bd28883c")

/**
 * Characteristic for bridging http requests via ble.file This Characteristic is for "PhoneToBoard"
 * direction (e.g. Sending to the board from android).
 */
private val DATA_RX_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("29e463e6-a210-4234-8d1d-4daf345b41de")

// The following characteristics define shared data between the board and the android app.
/**
 * Characteristic for the currently loaded game key. This can be a FEN or a id for the upstream api.
 * (e.g. lichess)
 */
private val GAME_KEY_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("d4f1e338-3396-4e72-a7d7-7c037fbcc0a1")

/**
 * Well known charakteristic for client configuration. It can be used to enable the notification
 * feature of BLE.
 */
private val CLIENT_CHARACTERISTIC_CONFIG_UUID: UUID =
    UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

private const val LOG_TAG = "BluetoothService"

class BluetoothService : Service() {
    inner class LocalBinder : Binder() {
        val service: BluetoothService
            get() = this@BluetoothService
    }

    private val bluetoothManager by lazy {
        getSystemService(BLUETOOTH_SERVICE) as BluetoothManager
    }
    private val adapter: BluetoothAdapter?
        get() = bluetoothManager.adapter
    private val scanner: BluetoothLeScanner?
        get() = adapter?.bluetoothLeScanner

    private val handler = Handler(Looper.getMainLooper())
    private val _uiState = MutableStateFlow(BleUiState())
    val uiState: StateFlow<BleUiState> = _uiState.asStateFlow()

    private var currentCallback: ScanCallback? = null
    private val tokenStore by lazy { LichessTokenStore(applicationContext) }
    private val bleBridge by lazy { BleHttpBridge(applicationContext, tokenStore) }
    private val binder = LocalBinder()

    override fun onCreate() {
        super.onCreate()
        val isEnabled = adapter?.isEnabled == true
        if (!isEnabled) {
            _uiState.update {
                it.copy(connectionState = "Bluetooth deaktiviert", canLoadGame = false)
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Toast.makeText(this, "Bluetooth Service gestartet", Toast.LENGTH_SHORT).show()
        return START_STICKY
    }

    override fun onDestroy() {
        stopScan()
        bleBridge.shutdown()
        super.onDestroy()
    }

    fun startScan() {
        val adapter = adapter
        val scanner = scanner
        if (adapter == null || scanner == null) {
            _uiState.update {
                it.copy(connectionState = "Bluetooth nicht verfügbar", canLoadGame = false)
            }
            return
        }
        if (!adapter.isEnabled) {
            _uiState.update {
                it.copy(connectionState = "Bluetooth deaktiviert", canLoadGame = false)
            }
            return
        }
        if (!hasPermissions(this, requiredPermissions())) {
            _uiState.update {
                it.copy(connectionState = "Berechtigungen fehlen", canLoadGame = false)
            }
            return
        }
        if (_uiState.value.scanning) return

        val callback =
            object : ScanCallback() {
                override fun onScanResult(callbackType: Int, result: ScanResult) {
                    val serviceUuids = result.scanRecord?.serviceUuids ?: emptyList()
                    if (ParcelUuid(SERVICE_UUID) !in serviceUuids) return
                    val address = result.device.address ?: return
                    // BLE scan callbacks happen off the main thread; push updates to UI state
                    // onto the main looper.
                    handler.post {
                        _uiState.update { state ->
                            val updated = state.devices.toMutableList()
                            val existingIndex = updated.indexOfFirst { it.address == address }
                            val newDevice =
                                SimpleDevice(result.device, address, result.device.name)
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
                    Log.e(LOG_TAG, "Scan failed: $errorCode")
                    _uiState.update {
                        it.copy(
                            connectionState = "Scan fehlgeschlagen ($errorCode)",
                            canLoadGame = false
                        )
                    }
                }
            }

        val settings =
            ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()

        try {
            scanner.startScan(null, settings, callback)
            currentCallback = callback
            _uiState.update {
                it.copy(scanning = true, connectionState = "Scanne...", canLoadGame = false)
            }
        } catch (se: SecurityException) {
            Log.e(LOG_TAG, "startScan without permission", se)
            _uiState.update {
                it.copy(
                    connectionState = "Scan fehlgeschlagen: Berechtigung fehlt",
                    canLoadGame = false
                )
            }
        }
    }

    fun stopScan() {
        val callback = currentCallback ?: return
        try {
            scanner?.stopScan(callback)
            _uiState.update { it.copy(scanning = false, connectionState = "Scan gestoppt") }
        } catch (se: SecurityException) {
            Log.e(LOG_TAG, "stopScan without permission", se)
            _uiState.update {
                it.copy(
                    connectionState = "Stop fehlgeschlagen: Berechtigung fehlt",
                    canLoadGame = false
                )
            }
        } finally {
            currentCallback = null
        }
    }

    fun connect(device: BluetoothDevice) {
        bleBridge.connect(device) { state, canLoadGame ->
            _uiState.update { it.copy(connectionState = state, canLoadGame = canLoadGame) }
        }
    }

    fun loadGame(gameKey: String): Boolean = bleBridge.loadGame(gameKey)

    fun disconnect() {
        bleBridge.close()
        _uiState.update { it.copy(connectionState = "Getrennt", canLoadGame = false) }
    }
}

private enum class RequestMethod {
    GET,
    POST,
    STREAM;

    companion object {
        fun fromWire(value: String): RequestMethod? =
            when (value.lowercase()) {
                "get" -> GET
                "post" -> POST
                "stream" -> STREAM
                else -> null
            }
    }
}

private sealed interface BoardToPhone {
    data class Request(val id: Int, val method: RequestMethod, val url: String, val body: String?) :
        BoardToPhone

    data class Cancel(val id: Int) : BoardToPhone
    data class Ping(val id: Int) : BoardToPhone
}

private sealed interface PhoneToBoard {
    data class Response(val id: Int, val body: String) : PhoneToBoard
    data class StreamData(val id: Int, val chunk: String) : PhoneToBoard
    data class StreamClosed(val id: Int) : PhoneToBoard
    data class Pong(val id: Int) : PhoneToBoard
    data class Error(val id: Int?, val message: String) : PhoneToBoard
}

private fun decodeBoardToPhone(raw: String): BoardToPhone? {
    return try {
        Log.d(LOG_TAG, "Raw message received: $raw")

        val json = JSONObject(raw)
        val version = json.optInt("v", -1)
        if (version != PROTOCOL_VERSION) {
            Log.w(LOG_TAG, "Protocol version mismatch: $version")
            return null
        }

        when (json.optString("type")) {
            "request" -> {
                val id = json.getInt("id")
                val method = RequestMethod.fromWire(json.optString("method")) ?: return null
                val url = json.getString("url")
                val body = if (json.isNull("body")) null else json.optString("body")
                BoardToPhone.Request(id, method, url, body)
            }

            "cancel" -> BoardToPhone.Cancel(json.getInt("id"))
            "ping" -> BoardToPhone.Ping(json.getInt("id"))
            else -> null
        }
    } catch (e: Exception) {
        Log.e(LOG_TAG, "Failed to decode incoming frame", e)
        null
    }
}

private fun encodePhoneToBoard(msg: PhoneToBoard): ByteArray {
    val json = JSONObject()
    json.put("v", PROTOCOL_VERSION)

    when (msg) {
        is PhoneToBoard.Response -> {
            json.put("type", "response")
            json.put("id", msg.id)
            json.put("body", msg.body)
        }

        is PhoneToBoard.StreamData -> {
            json.put("type", "stream_data")
            json.put("id", msg.id)
            json.put("chunk", msg.chunk)
        }

        is PhoneToBoard.StreamClosed -> {
            json.put("type", "stream_closed")
            json.put("id", msg.id)
        }

        is PhoneToBoard.Pong -> {
            json.put("type", "pong")
            json.put("id", msg.id)
        }

        is PhoneToBoard.Error -> {
            json.put("type", "error")
            msg.id?.let { json.put("id", it) }
            json.put("message", msg.message)
        }
    }

    return (json.toString() + "\n").toByteArray(StandardCharsets.UTF_8)
}

private class BleHttpBridge(
    private val context: Context,
    private val tokenStore: LichessTokenStore,
) {
    private val maxChunkSize = 20
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val mainHandler = Handler(Looper.getMainLooper())
    private val pendingBuffer = StringBuilder()
    private val activeRequests = ConcurrentHashMap<Int, Job>()
    private val writeMutex = Mutex()

    private var gatt: BluetoothGatt? = null
    private var rxCharacteristic: BluetoothGattCharacteristic? = null
    private var txCharacteristic: BluetoothGattCharacteristic? = null
    private var gameLoadCharacteristic: BluetoothGattCharacteristic? = null



    @SuppressLint("MissingPermission")
    fun connect(device: BluetoothDevice, onStateChange: (String, Boolean) -> Unit) {
        close()
        postState(onStateChange, "Verbinde mit ${device.address}...", false)

        gatt = device.connectGatt(context, false, createCallback(onStateChange, onCharacteristicWrite))
    }

    fun close() {
        cancelAllRequests()
        pendingBuffer.clear()
        rxCharacteristic = null
        txCharacteristic = null
        gameLoadCharacteristic = null
        gatt?.close()
        gatt = null
    }

    fun shutdown() {
        close()
        scope.cancel()
    }

    private fun cancelAllRequests() {
        activeRequests.values.forEach { it.cancel() }
        activeRequests.clear()
    }

    private fun postState(
        onStateChange: (String, Boolean) -> Unit,
        state: String,
        canLoadGame: Boolean
    ) {
        mainHandler.post { onStateChange(state, canLoadGame) }
    }

    private fun createCallback(
        onStateChange: (String, Boolean) -> Unit,
        onCharacteristicWrite: Unit
    ) =
        object : BluetoothGattCallback() {
            override fun onConnectionStateChange(
                gatt: BluetoothGatt,
                status: Int,
                newState: Int
            ) {
                val state =
                    when (newState) {
                        BluetoothProfile.STATE_CONNECTED ->
                            "Verbunden mit ${gatt.device.address}"

                        BluetoothProfile.STATE_CONNECTING -> "Verbindet..."
                        BluetoothProfile.STATE_DISCONNECTING -> "Trenne..."
                        BluetoothProfile.STATE_DISCONNECTED -> "Getrennt"
                        else -> "Status: $newState"
                    }
                postState(onStateChange, state, false)
                if (newState == BluetoothProfile.STATE_CONNECTED) {
                    gatt.discoverServices()
                }
                if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                    cancelAllRequests()
                    gatt.close()
                    gameLoadCharacteristic = null
                }
            }

            override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                val service = gatt.getService(SERVICE_UUID)
                if (service == null) {
                    Log.e(LOG_TAG, "Benötigter BLE Service nicht gefunden")
                    postState(onStateChange, "Service fehlt", false)
                    return
                }

                txCharacteristic = service.getCharacteristic(DATA_TX_CHARACTERISTIC_UUID)
                rxCharacteristic = service.getCharacteristic(DATA_RX_CHARACTERISTIC_UUID)
                gameLoadCharacteristic = service.getCharacteristic(GAME_KEY_CHARACTERISTIC_UUID)
                if (txCharacteristic == null ||
                    rxCharacteristic == null ||
                    gameLoadCharacteristic == null
                ) {
                    Log.e(LOG_TAG, "Charakteristiken nicht gefunden")
                    postState(onStateChange, "Charakteristik fehlt", false)
                    return
                }

                enableNotifications(gatt, txCharacteristic!!)
                postState(onStateChange, "Verbunden und bereit", true)
                Log.d(LOG_TAG, "connected to ble")
            }

            override fun onCharacteristicChanged(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                value: ByteArray
            ) {
                Log.d(LOG_TAG, "received characteristic change: $value")
                if (characteristic.uuid != DATA_TX_CHARACTERISTIC_UUID) return
                handleIncomingData(value, onStateChange)
            }

            override fun onCharacteristicWrite(
                gatt: BluetoothGatt?,
                characteristic: BluetoothGattCharacteristic?,
                status: Int
            ) {
               return onCharacteristicWrite(gatt, characteristic, status)
            }
        }

    private fun enableNotifications(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic
    ) {
        val notificationSet = gatt.setCharacteristicNotification(characteristic, true)
        if (!notificationSet) {
            Log.w(LOG_TAG, "setCharacteristicNotification fehlgeschlagen")
        }

        val descriptor = characteristic.getDescriptor(CLIENT_CHARACTERISTIC_CONFIG_UUID)
        if (descriptor != null) {
            gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
        } else {
            Log.w(LOG_TAG, "CCCD Descriptor nicht gefunden")
        }
    }

    private fun handleIncomingData(data: ByteArray, onStateChange: (String, Boolean) -> Unit) {
        pendingBuffer.append(String(data, StandardCharsets.UTF_8))
        var newlineIndex = nextDelimiterIndex()
        while (newlineIndex >= 0) {
            val frame = pendingBuffer.substring(0, newlineIndex).trim()
            pendingBuffer.delete(0, newlineIndex + 1)
            if (frame.isNotEmpty()) {
                val decoded = decodeBoardToPhone(frame)
                if (decoded != null) {
                    dispatch(decoded, onStateChange)
                } else {
                    scope.launch {
                        sendHttp(PhoneToBoard.Error(id = null, message = "Ungültiger Frame"))
                    }
                }
            }
            newlineIndex = nextDelimiterIndex()
        }
    }

    private fun nextDelimiterIndex(): Int {
        val lf = pendingBuffer.indexOf("\n")
        val cr = pendingBuffer.indexOf("\r")
        Log.d(LOG_TAG, "$pendingBuffer")
        return listOf(lf, cr).filter { it >= 0 }.minOrNull() ?: -1
    }

    private fun dispatch(msg: BoardToPhone, onStateChange: (String, Boolean) -> Unit) {
        when (msg) {
            is BoardToPhone.Ping -> scope.launch { sendHttp(PhoneToBoard.Pong(msg.id)) }
            is BoardToPhone.Cancel ->
                activeRequests
                    .remove(msg.id)
                    ?.cancel(CancellationException("Cancelled by board"))

            is BoardToPhone.Request -> {
                val job = scope.launch { runRequest(msg) }
                activeRequests[msg.id] = job
                job.invokeOnCompletion { activeRequests.remove(msg.id) }
            }
        }
    }

    private suspend fun runRequest(msg: BoardToPhone.Request) {
        Log.d(LOG_TAG, "Got request: $msg")
        try {
            when (msg.method) {
                RequestMethod.GET -> {
                    val body = executeHttp(msg.url, "GET", msg.body)
                    sendHttp(PhoneToBoard.Response(msg.id, body))
                }

                RequestMethod.POST -> {
                    val body = executeHttp(msg.url, "POST", msg.body)
                    sendHttp(PhoneToBoard.Response(msg.id, body))
                }

                RequestMethod.STREAM -> handleStream(msg.id, msg.url)
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "HTTP forwarding failed", e)
            if (e is CancellationException) return
            sendHttp(PhoneToBoard.Error(msg.id, e.message ?: "Unbekannter Fehler"))
        }
    }

    private fun executeHttp(url: String, method: String, body: String?): String {
        val parsedUrl = URL(url)
        val connection =
            (parsedUrl.openConnection() as HttpURLConnection).apply {
                requestMethod = method
                connectTimeout = 10_000
                readTimeout = 15_000
                doInput = true
            }
        addAuthorizationIfNeeded(connection, parsedUrl)

        if (method == "POST") {
            connection.doOutput = true
            body?.let {
                val payload = it.toByteArray(StandardCharsets.UTF_8)
                connection.setRequestProperty("Content-Type", "application/json")
                connection.outputStream.use { os -> os.write(payload) }
            }
        }

        val status = connection.responseCode
        val stream = if (status in 200..299) connection.inputStream else connection.errorStream
        val response =
            stream?.bufferedReader(StandardCharsets.UTF_8)?.use { reader -> reader.readText() }
                ?: ""
        connection.disconnect()
        if (status !in 200..299) {
            throw IOException("HTTP $status $response")
        }
        return response
    }

    private suspend fun handleStream(id: Int, url: String) {
        var connection: HttpURLConnection? = null
        try {
            val parsedUrl = URL(url)
            connection =
                (parsedUrl.openConnection() as HttpURLConnection).apply {
                    requestMethod = "GET"
                    connectTimeout = 10_000
                    readTimeout = 0
                    doInput = true
                }
            addAuthorizationIfNeeded(connection, parsedUrl)

            BufferedInputStream(connection.inputStream).use { input ->
                val scanner = Scanner(input)

                while (scanner.hasNextLine()) {
                    val line = scanner.nextLine()
                    Log.d(LOG_TAG, "received stream line $line")
                    sendHttp(PhoneToBoard.StreamData(id, line))
                }
            }
            sendHttp(PhoneToBoard.StreamClosed(id))
        } catch (e: CancellationException) {
            // cancelled by board, no response needed
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Streaming request failed", e)
            sendHttp(PhoneToBoard.Error(id, e.message ?: "Stream Fehler"))
        } finally {
            connection?.disconnect()
        }
    }

    @SuppressLint("MissingPermission")
    fun loadGame(gameKey: String): Boolean {
        val payload = gameKey.trim()
        if (payload.isEmpty()) return false
        val gatt = this.gatt ?: return false
        val characteristic = gameLoadCharacteristic ?: return false
        val bytes = payload.toByteArray(StandardCharsets.UTF_8)

        scope.launch {
            sendCharacteristic(
                gatt,
                characteristic,
                bytes,
            )
        }
        return true
    }

    private suspend fun sendHttp(msg: PhoneToBoard) {
        val gatt = this.gatt ?: return
        val characteristic = rxCharacteristic ?: return

        sendCharacteristic(gatt, characteristic, encodePhoneToBoard(msg))
    }

    private suspend fun sendCharacteristic(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, payload: ByteArray) {
        // TODO: This function must check via the callback if the data is sent correctly.
        // Only if this is valid and maybe with timeout - send next chunk

        Log.d(LOG_TAG, "send message ${payload.decodeToString()}")
        val chunks = payload.asList().chunked(this.maxChunkSize).map { it.toByteArray() }

        writeMutex.withLock {
            for (chunk in chunks) {
                Log.d(LOG_TAG, "send chunk of message ${chunk.decodeToString()}")

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    gatt.writeCharacteristic(
                        characteristic,
                        chunk,
                        BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                    )
                } else {
                    @Suppress("DEPRECATION")
                    val ok = gatt.writeCharacteristic(
                        characteristic.apply {
                            value = chunk
                            writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                        }
                    )
                    if (!ok) {
                        Log.e(LOG_TAG, "writeCharacteristic fehlgeschlagen")
                    }
                }
            }
        }
    }

    private fun addAuthorizationIfNeeded(connection: HttpURLConnection, url: URL) {
        val token = tokenStore.getToken()
        if (token != null && url.host.contains("lichess.org")) {
            connection.setRequestProperty("Authorization", "Bearer $token")
        }
    }
}

fun requiredPermissions(): List<String> {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        listOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT)
    } else {
        listOf(Manifest.permission.ACCESS_FINE_LOCATION)
    }
}

fun hasPermissions(context: Context, permissions: List<String>): Boolean {
    return permissions.all { permission ->
        ContextCompat.checkSelfPermission(context, permission) ==
                PackageManager.PERMISSION_GRANTED
    }
}
