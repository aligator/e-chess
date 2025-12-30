package me.aligator.e_chess

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.location.LocationManager
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.ui.theme.EChessTheme
import kotlinx.coroutines.flow.collectLatest
import me.aligator.e_chess.service.BleUiState
import me.aligator.e_chess.service.BluetoothService
import me.aligator.e_chess.service.SimpleDevice
import me.aligator.e_chess.service.hasPermissions
import me.aligator.e_chess.service.requiredPermissions

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
    val isPreview = LocalInspectionMode.current

    var bluetoothService by remember { mutableStateOf<BluetoothService?>(null) }
    var uiState by remember { mutableStateOf(BleUiState()) }
    var permissionsGranted by rememberSaveable { mutableStateOf(false) }
    var locationEnabled by rememberSaveable { mutableStateOf(isLocationEnabled(context)) }

    val permissionLauncher = rememberPermissionLauncher(
        requiredPermissions = requiredPermissions(),
        onResult = { granted -> permissionsGranted = granted }
    )

    val enableBtLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) {
        uiState = uiState.copy(connectionState = "Bluetooth aktiviert")
    }

    val locationSettingsLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) {
        locationEnabled = isLocationEnabled(context)
    }

    LaunchedEffect(isPreview) {
        if (!isPreview) {
            permissionLauncher.launch()
        }
    }
    LaunchedEffect(bluetoothService) {
        val service = bluetoothService ?: return@LaunchedEffect
        service.uiState.collectLatest { state -> uiState = state }
    }
    DisposableEffect(permissionsGranted, isPreview) {
        if (isPreview) return@DisposableEffect onDispose { }
        val connection = object : android.content.ServiceConnection {
            override fun onServiceConnected(name: ComponentName?, binder: android.os.IBinder?) {
                val service = (binder as? BluetoothService.LocalBinder)?.service
                bluetoothService = service
                service?.uiState?.value?.let { uiState = it }
            }

            override fun onServiceDisconnected(name: ComponentName?) {
                bluetoothService = null
            }
        }
        val intent = Intent(context, BluetoothService::class.java)
        // Start service so it survives activity recreation; binding alone would kill it on rotation.
        context.startService(intent)
        val bound = context.bindService(intent, connection, Context.BIND_AUTO_CREATE)
        if (!bound) {
            Log.e("Ble", "BluetoothService konnte nicht gebunden werden")
        }
        onDispose { if (bound) context.unbindService(connection) }
    }

    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val textPadding = Modifier.padding(innerPadding)
        when {
            uiState.connectionState == "Bluetooth deaktiviert" -> Button(
                onClick = { enableBtLauncher.launch(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)) },
                modifier = textPadding
            ) { Text("Bluetooth aktivieren") }

            uiState.connectionState == "Bluetooth nicht verfügbar" -> Text(
                "Bluetooth nicht verfügbar",
                modifier = textPadding
            )

            permissionsGranted.not() -> Text("Berechtigungen erforderlich", modifier = textPadding)
            locationEnabled.not() -> Button(
                onClick = { locationSettingsLauncher.launch(Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)) },
                modifier = textPadding
            ) { Text("Standort einschalten") }

            bluetoothService == null -> Text("Bluetooth Service wird verbunden...", modifier = textPadding)

            else -> {
                BleContent(
                    scanning = uiState.scanning,
                    connectionState = uiState.connectionState,
                    devices = uiState.devices,
                    onStartScan = {
                        if (permissionsGranted && hasPermissions(
                                context,
                                requiredPermissions()
                            )
                        ) {
                            bluetoothService?.startScan()
                        } else {
                            uiState = uiState.copy(connectionState = "Berechtigungen fehlen")
                        }
                    },
                    onStopScan = {
                        bluetoothService?.stopScan()
                    },
                    onConnect = { device ->
                        bluetoothService?.connect(device)
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

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
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
