package me.aligator.e_chess.ui

import android.bluetooth.BluetoothAdapter
import android.content.ComponentName
import android.content.Intent
import android.provider.Settings
import android.util.Log
import android.widget.Toast
import androidx.activity.compose.LocalActivityResultRegistryOwner
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import me.aligator.e_chess.R
import me.aligator.e_chess.service.bluetooth.BluetoothService
import me.aligator.e_chess.service.bluetooth.SimpleDevice
import me.aligator.e_chess.service.bluetooth.hasPermissions
import me.aligator.e_chess.service.bluetooth.requiredPermissions


@Composable
fun BleScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val isPreview = LocalInspectionMode.current
    val activityResultOwner = LocalActivityResultRegistryOwner.current

    var bluetoothService by remember { mutableStateOf<BluetoothService?>(null) }
    val initialPermissionsGranted = remember { hasPermissions(context) }
    var permissionsGranted by rememberSaveable { mutableStateOf(initialPermissionsGranted) }
    var locationEnabled by rememberSaveable { mutableStateOf(isLocationEnabled(context)) }

    val permissionLauncher =
        rememberPermissionLauncher(
            requiredPermissions = requiredPermissions(),
            onResult = { granted -> permissionsGranted = granted }
        )

    val enableBtLauncher =
        if (activityResultOwner == null || isPreview) {
            null
        } else {
            rememberLauncherForActivityResult(
                ActivityResultContracts.StartActivityForResult()
            ) {
                bluetoothService?.ble?.checkBluetooth()
            }
        }

    val locationSettingsLauncher =
        if (activityResultOwner == null || isPreview) {
            null
        } else {
            rememberLauncherForActivityResult(
                ActivityResultContracts.StartActivityForResult()
            ) { locationEnabled = isLocationEnabled(context) }
        }

    LaunchedEffect(isPreview, initialPermissionsGranted) {
        if (!isPreview && !initialPermissionsGranted) {
            permissionLauncher()
        }
    }
    DisposableEffect(permissionsGranted, isPreview) {
        if (isPreview) return@DisposableEffect onDispose {}
        val connection =
            object : android.content.ServiceConnection {
                override fun onServiceConnected(
                    name: ComponentName?,
                    binder: android.os.IBinder?
                ) {
                    val service = (binder as? BluetoothService.LocalBinder)?.service
                    bluetoothService = service
                }

                override fun onServiceDisconnected(name: ComponentName?) {
                    bluetoothService = null
                }
            }
        val intent = Intent(context, BluetoothService::class.java)
        // Start service so it survives activity recreation; binding alone would kill it on
        // rotation.
        context.startService(intent)
        val bound =
            context.bindService(intent, connection, android.content.Context.BIND_AUTO_CREATE)
        if (!bound) {
            Log.e("Ble", "BluetoothService konnte nicht gebunden werden")
        }
        onDispose { if (bound) context.unbindService(connection) }
    }

    bluetoothService?.let { service ->
        val bleState by service.ble.bleState.collectAsState()
        BleScreenContent(
            modifier = modifier,
            bleState = bleState,
            permissionsGranted = permissionsGranted,
            locationEnabled = locationEnabled,
            onRequestEnableBt = {
                enableBtLauncher?.launch(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE))
                    ?: run {
                        bluetoothService!!.ble.checkBluetooth()
                    }
            },
            onOpenLocationSettings = {
                locationSettingsLauncher?.launch(Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS))
                    ?: run { locationEnabled = isLocationEnabled(context) }
            },
            onStartScan = {
                bluetoothService!!.ble.startScan()
            },
            onStopScan = { bluetoothService!!.ble.stopScan() },
            onConnect = { device: SimpleDevice -> bluetoothService!!.ble.connect(device) },
            onLoadGame = { gameKey ->
                val messageRes =
                    when {
                        bluetoothService?.chessBoardAction?.loadGame(gameKey) == true ->
                            R.string.load_game_sent

                        else -> R.string.load_game_failed
                    }
                Toast.makeText(context, context.getString(messageRes), Toast.LENGTH_SHORT).show()
            },
        )
    }

}
