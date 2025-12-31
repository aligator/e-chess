package me.aligator.e_chess.ui

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.content.ComponentName
import android.content.Intent
import android.provider.Settings
import android.util.Log
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import kotlinx.coroutines.flow.collectLatest
import me.aligator.e_chess.service.BleUiState
import me.aligator.e_chess.service.BluetoothService
import me.aligator.e_chess.service.hasPermissions
import me.aligator.e_chess.service.requiredPermissions
import me.aligator.e_chess.ui.BleScreenContent
import me.aligator.e_chess.ui.isLocationEnabled
import me.aligator.e_chess.ui.rememberPermissionLauncher

@Composable
fun BleScreen(modifier: Modifier = Modifier) {
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
            permissionLauncher()
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
        val bound = context.bindService(intent, connection, android.content.Context.BIND_AUTO_CREATE)
        if (!bound) {
            Log.e("Ble", "BluetoothService konnte nicht gebunden werden")
        }
        onDispose { if (bound) context.unbindService(connection) }
    }

    BleScreenContent(
        modifier = modifier,
        uiState = uiState,
        permissionsGranted = permissionsGranted,
        locationEnabled = locationEnabled,
        bluetoothServiceConnected = bluetoothService != null,
        onRequestEnableBt = { enableBtLauncher.launch(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)) },
        onOpenLocationSettings = {
            locationSettingsLauncher.launch(Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS))
        },
        onStartScan = {
            if (permissionsGranted && hasPermissions(context, requiredPermissions())) {
                bluetoothService?.startScan()
            } else {
                uiState = uiState.copy(connectionState = "Berechtigungen fehlen")
            }
        },
        onStopScan = { bluetoothService?.stopScan() },
        onConnect = { device: BluetoothDevice -> bluetoothService?.connect(device) },
    )
}
