package me.aligator.e_chess

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothManager
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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountBox
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Home
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import me.aligator.e_chess.ui.theme.EChessTheme

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
                        connectToDevice(context, device) { state ->
                            connectionState = state
                        }
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

@SuppressLint("MissingPermission")
private fun connectToDevice(
    context: Context,
    device: BluetoothDevice,
    onStateChange: (String) -> Unit,
) {
    onStateChange("Verbinde mit ${device.address}...")
    device.connectGatt(context, false, object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            val state = when (newState) {
                BluetoothGatt.STATE_CONNECTED -> "Verbunden mit ${gatt.device.address}"
                BluetoothGatt.STATE_CONNECTING -> "Verbindet..."
                BluetoothGatt.STATE_DISCONNECTING -> "Trenne..."
                BluetoothGatt.STATE_DISCONNECTED -> "Getrennt"
                else -> "Status: $newState"
            }
            onStateChange(state)
            if (newState == BluetoothGatt.STATE_CONNECTED) {
                gatt.discoverServices()
            }
            if (newState == BluetoothGatt.STATE_DISCONNECTED) {
                gatt.close()
            }
        }
    })
}

private data class SimpleDevice(
    val device: BluetoothDevice,
    val address: String,
    val name: String?,
)

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
