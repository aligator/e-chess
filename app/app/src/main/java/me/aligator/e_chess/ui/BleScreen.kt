package me.aligator.e_chess.ui

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import kotlinx.coroutines.flow.collectLatest
import me.aligator.e_chess.R
import me.aligator.e_chess.service.BleUiState
import me.aligator.e_chess.service.BluetoothService
import me.aligator.e_chess.service.hasPermissions
import me.aligator.e_chess.service.requiredPermissions

@Composable
fun BleScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val isPreview = LocalInspectionMode.current
    val activityResultOwner = LocalActivityResultRegistryOwner.current

    var bluetoothService by remember { mutableStateOf<BluetoothService?>(null) }
    var uiState by remember { mutableStateOf(BleUiState()) }
    val initialPermissionsGranted = remember { hasPermissions(context, requiredPermissions()) }
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
                    uiState =
                            uiState.copy(
                                    connectionState = "Bluetooth aktiviert",
                                    canLoadGame = false
                            )
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
    LaunchedEffect(bluetoothService) {
        val service = bluetoothService ?: return@LaunchedEffect
        service.uiState.collectLatest { state -> uiState = state }
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
                        service?.uiState?.value?.let { uiState = it }
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

    BleScreenContent(
            modifier = modifier,
            uiState = uiState,
            permissionsGranted = permissionsGranted,
            locationEnabled = locationEnabled,
            bluetoothServiceConnected = bluetoothService != null,
            onRequestEnableBt = {
                enableBtLauncher?.launch(Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE))
                        ?: run {
                            uiState =
                                    uiState.copy(
                                            connectionState = "Bluetooth aktiviert",
                                            canLoadGame = false
                                    )
                        }
            },
            onOpenLocationSettings = {
                locationSettingsLauncher?.launch(Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS))
                        ?: run { locationEnabled = isLocationEnabled(context) }
            },
            onStartScan = {
                if (permissionsGranted && hasPermissions(context, requiredPermissions())) {
                    bluetoothService?.startScan()
                } else {
                    uiState =
                            uiState.copy(
                                    connectionState = "Berechtigungen fehlen",
                                    canLoadGame = false
                            )
                }
            },
            onStopScan = { bluetoothService?.stopScan() },
            onConnect = { device: BluetoothDevice -> bluetoothService?.connect(device) },
            onLoadGame = { gameKey ->
                val cleanedKey = extractGameKey(gameKey)
                val messageRes =
                        when {
                            cleanedKey == null -> R.string.load_game_invalid_key
                            cleanedKey.length > 20 -> R.string.load_game_too_long
                            bluetoothService?.loadGame(cleanedKey) == true ->
                                    R.string.load_game_sent
                            else -> R.string.load_game_failed
                        }
                Toast.makeText(context, context.getString(messageRes), Toast.LENGTH_SHORT).show()
            },
    )
}

private fun extractGameKey(raw: String): String? {
    if (raw.isBlank()) return null
    val candidate = raw.trim().substringAfterLast("/")
    val match = Regex("([a-zA-Z0-9]{8,12})").find(candidate)?.groupValues?.getOrNull(1)
    return match
}
