package me.aligator.e_chess.ui

import android.bluetooth.BluetoothAdapter
import android.content.Context
import android.content.Intent
import android.location.LocationManager
import android.provider.Settings
import android.widget.Toast
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import me.aligator.e_chess.R
import me.aligator.e_chess.service.bluetooth.BluetoothService
import me.aligator.e_chess.service.bluetooth.ConnectionStep
import me.aligator.e_chess.ui.components.ConnectionStatusCard
import me.aligator.e_chess.ui.components.DeviceScanner
import me.aligator.e_chess.ui.components.GameLoader

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

@Composable
fun BleScreen(
    permissionsGranted: Boolean,
    modifier: Modifier = Modifier,
    bluetoothService: BluetoothService? = null,
    viewModel: BleViewModel = viewModel()
) {
    val context = LocalContext.current

    var locationEnabled by remember { mutableStateOf(isLocationEnabled(context)) }
    val uiState by viewModel.uiState.collectAsState()

    LaunchedEffect(bluetoothService) {
        viewModel.setBluetoothService(bluetoothService)
    }

    LaunchedEffect(uiState.isConnected) {
        if (uiState.isConnected && uiState.bleState.step == ConnectionStep.SCANNING) {
            viewModel.stopScan()
        }
    }

    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val contentModifier = Modifier.padding(innerPadding)

        when {
            uiState.bleState.step == ConnectionStep.DISABLED ->
                Button(
                    onClick = {
                        val intent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
                        context.startActivity(intent)
                    },
                    modifier = contentModifier
                ) {
                    Text(stringResource(R.string.bluetooth_enable))
                }

            uiState.bleState.step == ConnectionStep.UNAVAILABLE ->
                Text(
                    text = stringResource(R.string.bluetooth_unavailable),
                    modifier = contentModifier
                )

            !permissionsGranted ->
                Text(
                    text = stringResource(R.string.permissions_required),
                    modifier = contentModifier
                )

            !locationEnabled ->
                Button(
                    onClick = {
                        val intent = Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)
                        context.startActivity(intent)
                    },
                    modifier = contentModifier
                ) {
                    Text(stringResource(R.string.location_button))
                }

            else ->
                BleContent(
                    uiState = uiState,
                    onStartScan = viewModel::startScan,
                    onStopScan = viewModel::stopScan,
                    onConnect = viewModel::connect,
                    onDisconnect = viewModel::disconnect,
                    onLoadGame = { gameKey ->
                        viewModel.loadGame(gameKey)
                        if (!uiState.isLoadingGame) {
                            val message = context.getString(R.string.load_game_sent)
                            Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
                        }
                    },
                    onFetchGames = viewModel::fetchGames,
                    onGameKeyChanged = viewModel::setSelectedGameKey,
                    modifier = contentModifier
                )
        }
    }
}

@Composable
private fun BleContent(
    uiState: BleUiState,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (me.aligator.e_chess.service.bluetooth.SimpleDevice) -> Unit,
    onDisconnect: () -> Unit,
    onLoadGame: (String) -> Unit,
    onFetchGames: () -> Unit,
    onGameKeyChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize()) {
        ConnectionStatusCard(
            connectionState = uiState.bleState.connectedDevice,
            onDisconnect = onDisconnect
        )

        if (uiState.isConnected) {
            GameLoader(
                availableGames = uiState.availableGames,
                selectedGameKey = uiState.selectedGameKey,
                onGameKeyChanged = onGameKeyChanged,
                onLoadGame = onLoadGame,
                onFetchGames = onFetchGames,
                isLoadingGames = uiState.isLoadingGames,
                isLoadingGame = uiState.isLoadingGame
            )
        } else {
            DeviceScanner(
                scanning = uiState.bleState.step == ConnectionStep.SCANNING,
                devices = uiState.bleState.devices,
                connectedDevice = uiState.bleState.connectedDevice,
                onStartScan = onStartScan,
                onStopScan = onStopScan,
                onConnect = onConnect
            )
        }
    }
}
