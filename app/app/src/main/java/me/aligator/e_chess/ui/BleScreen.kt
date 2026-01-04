package me.aligator.e_chess.ui

import android.bluetooth.BluetoothAdapter
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.location.LocationManager
import android.provider.Settings
import android.util.Log
import android.widget.Toast
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import kotlinx.coroutines.launch
import me.aligator.e_chess.R
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.LichessApi
import me.aligator.e_chess.service.bluetooth.BluetoothService
import me.aligator.e_chess.service.bluetooth.SimpleDevice

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

private const val STANDARD_GAME_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"


@Composable
fun BleScreen(
    permissionsGranted: Boolean,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val isPreview = LocalInspectionMode.current

    var bluetoothService by remember { mutableStateOf<BluetoothService?>(null) }
    var locationEnabled by remember { mutableStateOf<Boolean>(isLocationEnabled(context)) }
    var availableGames by remember { mutableStateOf<List<GameOption>>(emptyList()) }

    val lichessApi = remember { LichessApi(context) }

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
                val intent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
                context.startActivity(intent)
            },
            onOpenLocationSettings = {
                val intent = Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)
                context.startActivity(intent)
            },
            onStartScan = {
                bluetoothService!!.ble.startScan()
            },
            onStopScan = { bluetoothService!!.ble.stopScan() },
            onConnect = { device: SimpleDevice -> bluetoothService!!.ble.connect(device) },
            onLoadGame = { gameKey ->
                // Convert "standard" to the actual FEN string
                val actualGameKey = if (gameKey == "standard") {
                    STANDARD_GAME_FEN
                } else {
                    gameKey
                }

                val messageRes =
                    when {
                        bluetoothService?.chessBoardAction?.loadGame(actualGameKey) == true ->
                            R.string.load_game_sent

                        else -> R.string.load_game_failed
                    }
                Toast.makeText(context, context.getString(messageRes), Toast.LENGTH_SHORT).show()
            },
            onFetchGames = {
                kotlinx.coroutines.CoroutineScope(kotlinx.coroutines.Dispatchers.Main).launch {
                    availableGames = lichessApi.getOngoingGames()
                }
            },
            availableGames = availableGames,
        )
    }

}
