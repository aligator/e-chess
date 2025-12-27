package me.aligator.e_chess

import android.Manifest
import android.annotation.SuppressLint
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
import android.location.LocationManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import me.aligator.e_chess.ui.theme.EChessTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.isActive
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject
import java.io.BufferedInputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.nio.charset.StandardCharsets
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.coroutines.coroutineContext

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            EChessTheme {
                EChessApp()
            }
        }
    }
}

@PreviewScreenSizes
@Composable
fun EChessApp() {
    BleScreen()
}


@Composable
private fun BleScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
    val adapter = bluetoothManager?.adapter
    val bleBridge = remember { BleHttpBridge(context.applicationContext) }

    var permissionsGranted by rememberSaveable { mutableStateOf(false) }
    var scanning by rememberSaveable { mutableStateOf(false) }
    var connectionState by rememberSaveable { mutableStateOf("Keine Verbindung") }
    val scanResults = remember { mutableStateListOf<SimpleDevice>() }
    var locationEnabled by rememberSaveable { mutableStateOf(isLocationEnabled(context)) }

    val permissionLauncher = rememberPermissionLauncher(
        requiredPermissions = requiredPermissions(),
        onResult = { granted -> permissionsGranted = granted }
    )

    val enableBtLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) {
        if (adapter?.isEnabled == true) {
            connectionState = "Bluetooth aktiviert"
        }
    }

    val locationSettingsLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) {
        locationEnabled = isLocationEnabled(context)
    }

    val scanner = adapter?.bluetoothLeScanner
    var currentCallback by remember { mutableStateOf<ScanCallback?>(null) }

    DisposableEffect(scanner, permissionsGranted) {
        onDispose {
            currentCallback?.let { callback ->
                if (permissionsGranted && hasPermissions(context, requiredPermissions())) {
                    try {
                        scanner?.stopScan(callback)
                    } catch (se: SecurityException) {
                        Log.e("Ble", "stopScan without permission", se)
                    }
                }
            }
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            bleBridge.close()
        }
    }

    LaunchedEffect(Unit) {
        permissionLauncher.launch()
    }

    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val textPadding = Modifier.padding(innerPadding)
        when {
            adapter == null -> Text("Bluetooth nicht verfügbar", modifier = textPadding)
            adapter.isEnabled.not() -> Button(
                onClick = { enableBtLauncher.launch(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)) },
                modifier = textPadding
            ) { Text("Bluetooth aktivieren") }

            permissionsGranted.not() -> Text("Berechtigungen erforderlich", modifier = textPadding)

            locationEnabled.not() -> Button(
                onClick = { locationSettingsLauncher.launch(Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)) },
                modifier = textPadding
            ) { Text("Standort einschalten") }

            else -> {
                BleContent(
                    scanning = scanning,
                    connectionState = connectionState,
                    devices = scanResults,
                    onStartScan = {
                        if (scanner != null && permissionsGranted && hasPermissions(context, requiredPermissions())) {
                            connectionState = "Scanne..."
                            startScan(scanner, scanResults) { callback ->
                                currentCallback = callback
                                scanning = true
                            }
                        }
                    },
                    onStopScan = {
                        if (permissionsGranted && hasPermissions(context, requiredPermissions())) {
                            currentCallback?.let { callback ->
                                try {
                                    scanner?.stopScan(callback)
                                    connectionState = "Scan gestoppt"
                                } catch (se: SecurityException) {
                                    Log.e("Ble", "stopScan without permission", se)
                                    connectionState = "Stop fehlgeschlagen: Berechtigung fehlt"
                                }
                            }
                        } else {
                            connectionState = "Stop übersprungen: Berechtigung fehlt"
                        }
                        scanning = false
                    },
                    onConnect = { device ->
                        bleBridge.connect(device) { state -> connectionState = state }
                    },
                    modifier = textPadding
                )
            }
        }
    }
}

@Composable
private fun BleContent(
    scanning: Boolean,
    connectionState: String,
    devices: List<SimpleDevice>,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (BluetoothDevice) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(modifier = modifier.fillMaxSize()) {
        item {
            Text(
                text = "Status: $connectionState",
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.padding(16.dp)
            )
            Button(
                onClick = if (scanning) onStopScan else onStartScan,
                modifier = Modifier.padding(horizontal = 16.dp)
            ) { Text(if (scanning) "Scan stoppen" else "Nach Geräten suchen") }
        }
        items(devices) { device ->
            Card(
                modifier = Modifier
                    .padding(horizontal = 16.dp, vertical = 8.dp)
                    .fillMaxWidth(),
                colors = CardDefaults.cardColors()
            ) {
                Text(
                    text = device.name ?: "Unbekannt",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                )
                Text(
                    text = device.address,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(horizontal = 16.dp)
                )
                Button(
                    onClick = { onConnect(device.device) },
                    modifier = Modifier.padding(16.dp)
                ) { Text("Verbinden") }
            }
        }
    }
}


@Preview(showBackground = true)
@Composable
fun BleScreenPreview() {
    EChessTheme {
        BleScreen()
    }
}

@SuppressLint("MissingPermission")
private fun startScan(
    scanner: BluetoothLeScanner,
    devices: MutableList<SimpleDevice>,
    onStarted: (ScanCallback) -> Unit,
) {
    val mainHandler = Handler(Looper.getMainLooper())

    val callback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            val address = result.device.address ?: return
            // BLE scan callbacks happen off the main thread; push updates to UI state onto the main looper.
            mainHandler.post {
                val existingIndex = devices.indexOfFirst { it.address == address }
                if (existingIndex >= 0) {
                    devices[existingIndex] = SimpleDevice(result.device, address, result.device.name)
                } else {
                    devices.add(SimpleDevice(result.device, address, result.device.name))
                }
            }
        }

        override fun onScanFailed(errorCode: Int) {
            Log.e("Ble", "Scan failed: $errorCode")
        }
    }

    val settings = ScanSettings.Builder()
        .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
        .build()

    try {
        scanner.startScan(null, settings, callback)
        onStarted(callback)
    } catch (se: SecurityException) {
        Log.e("Ble", "startScan without permission", se)
    }
}

private data class SimpleDevice(
    val device: BluetoothDevice,
    val address: String,
    val name: String?,
)

private const val PROTOCOL_VERSION = 1
private val SERVICE_UUID: UUID = UUID.fromString("b4d75b6c-7284-4268-8621-6e3cef3c6ac4")
private val DATA_TX_CHAR_UUID: UUID = UUID.fromString("aa8381af-049a-46c2-9c92-1db7bd28883c")
private val DATA_RX_CHAR_UUID: UUID = UUID.fromString("29e463e6-a210-4234-8d1d-4daf345b41de")
private val CLIENT_CHARACTERISTIC_CONFIG_UUID: UUID =
    UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
private const val TAG_BLE = "Ble"

private enum class RequestMethod {
    GET,
    POST,
    STREAM;

    companion object {
        fun fromWire(value: String): RequestMethod? = when (value.lowercase()) {
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
        Log.d(TAG_BLE, "Raw message received: ${raw}")

        val json = JSONObject(raw)
        val version = json.optInt("v", -1)
        if (version != PROTOCOL_VERSION) {
            Log.w(TAG_BLE, "Protocol version mismatch: $version")
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
        Log.e(TAG_BLE, "Failed to decode incoming frame", e)
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

private class BleHttpBridge(private val context: Context) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val mainHandler = Handler(Looper.getMainLooper())
    private val pendingBuffer = StringBuilder()
    private val activeRequests = ConcurrentHashMap<Int, Job>()
    private val writeMutex = Mutex()

    private var gatt: BluetoothGatt? = null
    private var rxCharacteristic: BluetoothGattCharacteristic? = null
    private var txCharacteristic: BluetoothGattCharacteristic? = null

    @SuppressLint("MissingPermission")
    fun connect(device: BluetoothDevice, onStateChange: (String) -> Unit) {
        close()
        postState(onStateChange, "Verbinde mit ${device.address}...")
        gatt = device.connectGatt(context, false, createCallback(onStateChange))
    }

    fun close() {
        cancelAllRequests()
        pendingBuffer.clear()
        rxCharacteristic = null
        txCharacteristic = null
        gatt?.close()
        gatt = null
    }

    private fun cancelAllRequests() {
        activeRequests.values.forEach { it.cancel() }
        activeRequests.clear()
    }

    private fun postState(onStateChange: (String) -> Unit, state: String) {
        mainHandler.post { onStateChange(state) }
    }

    private fun createCallback(onStateChange: (String) -> Unit) = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            val state = when (newState) {
                BluetoothProfile.STATE_CONNECTED -> "Verbunden mit ${gatt.device.address}"
                BluetoothProfile.STATE_CONNECTING -> "Verbindet..."
                BluetoothProfile.STATE_DISCONNECTING -> "Trenne..."
                BluetoothProfile.STATE_DISCONNECTED -> "Getrennt"
                else -> "Status: $newState"
            }
            postState(onStateChange, state)
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                gatt.discoverServices()
            }
            if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                cancelAllRequests()
                gatt.close()
            }
        }

        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            val service = gatt.getService(SERVICE_UUID)
            if (service == null) {
                Log.e(TAG_BLE, "Benötigter BLE Service nicht gefunden")
                postState(onStateChange, "Service fehlt")
                return
            }

            gatt.requestMtu(1024);

            txCharacteristic = service.getCharacteristic(DATA_TX_CHAR_UUID)
            rxCharacteristic = service.getCharacteristic(DATA_RX_CHAR_UUID)
            if (txCharacteristic == null || rxCharacteristic == null) {
                Log.e(TAG_BLE, "Charakteristiken nicht gefunden")
                postState(onStateChange, "Charakteristik fehlt")
                return
            }

            enableNotifications(gatt, txCharacteristic!!)
            postState(onStateChange, "Verbunden und bereit")
            Log.d(TAG_BLE, "connected to ble")

        }

        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray
        ) {
            Log.d(TAG_BLE, "received characteristic change: $value")
            if (characteristic.uuid != DATA_TX_CHAR_UUID) return
            handleIncomingData(value, onStateChange)
        }
    }

    private fun enableNotifications(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {


        val notificationSet = gatt.setCharacteristicNotification(characteristic, true)
        if (!notificationSet) {
            Log.w(TAG_BLE, "setCharacteristicNotification fehlgeschlagen")
        }

        val descriptor = characteristic.getDescriptor(CLIENT_CHARACTERISTIC_CONFIG_UUID)
        if (descriptor != null) {
            gatt.writeDescriptor(descriptor,  BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
        } else {
            Log.w(TAG_BLE, "CCCD Descriptor nicht gefunden")
        }
    }

    private fun handleIncomingData(data: ByteArray, onStateChange: (String) -> Unit) {
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
                    scope.launch { send(PhoneToBoard.Error(id = null, message = "Ungültiger Frame")) }
                }
            }
            newlineIndex = nextDelimiterIndex()
        }
    }

    private fun nextDelimiterIndex(): Int {
        val lf = pendingBuffer.indexOf("\n")
        val cr = pendingBuffer.indexOf("\r")
        Log.d(TAG_BLE, "$pendingBuffer")
        return listOf(lf, cr).filter { it >= 0 }.minOrNull() ?: -1
    }

    private fun dispatch(msg: BoardToPhone, onStateChange: (String) -> Unit) {
        when (msg) {
            is BoardToPhone.Ping -> scope.launch { send(PhoneToBoard.Pong(msg.id)) }
            is BoardToPhone.Cancel -> activeRequests.remove(msg.id)
                ?.cancel(CancellationException("Cancelled by board"))

            is BoardToPhone.Request -> {
                val job = scope.launch { runRequest(msg) }
                activeRequests[msg.id] = job
                job.invokeOnCompletion { activeRequests.remove(msg.id) }
            }
        }
    }

    private suspend fun runRequest(msg: BoardToPhone.Request) {
        Log.d(TAG_BLE, "Got request: $msg");
        try {
            when (msg.method) {
                RequestMethod.GET -> {
                    val body = executeHttp(msg.url, "GET", msg.body)
                    send(PhoneToBoard.Response(msg.id, body))
                }

                RequestMethod.POST -> {
                    val body = executeHttp(msg.url, "POST", msg.body)
                    send(PhoneToBoard.Response(msg.id, body))
                }

                RequestMethod.STREAM -> handleStream(msg.id, msg.url)
            }
        } catch (e: Exception) {
            Log.e(TAG_BLE, "HTTP forwarding failed", e)
            if (e is CancellationException) return
            send(PhoneToBoard.Error(msg.id, e.message ?: "Unbekannter Fehler"))
        }
    }

    private fun executeHttp(url: String, method: String, body: String?): String {
        val connection = (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 15_000
            doInput = true
        }

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
            connection = (URL(url).openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = 10_000
                readTimeout = 0
                doInput = true
            }

            BufferedInputStream(connection.inputStream).use { input ->
                val buffer = ByteArray(1024)
                while (coroutineContext.isActive) {
                    val read = input.read(buffer)
                    if (read <= 0) break
                    val chunk = String(buffer, 0, read, StandardCharsets.UTF_8)
                    send(PhoneToBoard.StreamData(id, chunk))
                }
            }
            send(PhoneToBoard.StreamClosed(id))
        } catch (e: CancellationException) {
            // cancelled by board, no response needed
        } catch (e: Exception) {
            Log.e(TAG_BLE, "Streaming request failed", e)
            send(PhoneToBoard.Error(id, e.message ?: "Stream Fehler"))
        } finally {
            connection?.disconnect()
        }
    }

    private suspend fun send(msg: PhoneToBoard) {
        val gatt = this.gatt ?: return
        val characteristic = rxCharacteristic ?: return
        val payload = encodePhoneToBoard(msg)

        writeMutex.withLock {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                gatt.writeCharacteristic(
                    characteristic,
                    payload,
                    BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                )
            } else {
                @Suppress("DEPRECATION")
                val ok = gatt.writeCharacteristic(
                    characteristic.apply {
                        value = payload
                        writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                    }
                )
                if (!ok) {
                    Log.e(TAG_BLE, "writeCharacteristic fehlgeschlagen")
                }
            }
        }
    }
}

private fun requiredPermissions(): List<String> {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        listOf(
            Manifest.permission.BLUETOOTH_SCAN,
            Manifest.permission.BLUETOOTH_CONNECT
        )
    } else {
        listOf(Manifest.permission.ACCESS_FINE_LOCATION)
    }
}

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

private fun hasPermissions(context: Context, permissions: List<String>): Boolean {
    return permissions.all { permission ->
        ContextCompat.checkSelfPermission(context, permission) == android.content.pm.PackageManager.PERMISSION_GRANTED
    }
}

@Composable
private fun rememberPermissionLauncher(
    requiredPermissions: List<String>,
    onResult: (Boolean) -> Unit,
): PermissionLauncher {
    val launcher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { grantResults ->
        val granted = requiredPermissions.all { permission ->
            grantResults[permission] == true
        }
        onResult(granted)
    }
    return PermissionLauncher { launcher.launch(requiredPermissions.toTypedArray()) }
}

private class PermissionLauncher(private val launcher: () -> Unit) {
    fun launch() = launcher()
}
