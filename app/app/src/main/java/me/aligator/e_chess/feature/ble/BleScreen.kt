package me.aligator.e_chess.feature.ble

import android.bluetooth.BluetoothAdapter
import android.content.Context
import android.content.Intent
import android.location.LocationManager
import android.provider.Settings
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Button
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import me.aligator.e_chess.R
import me.aligator.e_chess.platform.ble.BoardBleService
import me.aligator.e_chess.platform.ble.ConnectionStep
import me.aligator.e_chess.feature.ble.components.ConnectionStatusCard
import me.aligator.e_chess.feature.ble.components.DeviceScanner
import me.aligator.e_chess.feature.ble.components.GameLoader
import me.aligator.e_chess.ui.UiEvent
import me.aligator.e_chess.ui.UiMessage

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

private sealed class BleScreenState {
    data class Requirement(
        val title: String,
        val description: String,
        val actionLabel: String? = null,
        val action: (() -> Unit)? = null,
    ) : BleScreenState()

    data class Ready(
        val showSetupBanner: Boolean = true
    ) : BleScreenState()
}

@Composable
fun BleScreen(
    permissionsGranted: Boolean,
    modifier: Modifier = Modifier,
    bluetoothService: BoardBleService? = null,
    viewModel: BleViewModel = viewModel()
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val snackbarHostState = remember { SnackbarHostState() }

    var locationEnabled by remember { mutableStateOf(isLocationEnabled(context)) }
    val uiState by viewModel.uiState.collectAsState()

    LaunchedEffect(bluetoothService) {
        viewModel.setBluetoothService(bluetoothService)
    }

    androidx.compose.runtime.DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                locationEnabled = isLocationEnabled(context)
            }
            if (event == Lifecycle.Event.ON_PAUSE) {
                viewModel.stopScan()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    LaunchedEffect(uiState.isConnected) {
        if (uiState.isConnected && uiState.bleState.step == ConnectionStep.SCANNING) {
            viewModel.stopScan()
        }
    }

    LaunchedEffect(Unit) {
        viewModel.events.collect { event ->
            when (event) {
                is UiEvent.Snackbar -> {
                    val message = when (val msg = event.message) {
                        is UiMessage.Res -> context.getString(msg.id, *msg.args.toTypedArray())
                        is UiMessage.Text -> msg.value
                    }
                    snackbarHostState.showSnackbar(message)
                }
            }
        }
    }

    val screenState = when {
        uiState.bleState.step == ConnectionStep.DISABLED ->
            BleScreenState.Requirement(
                title = stringResource(R.string.bluetooth_enable),
                description = stringResource(R.string.ble_bluetooth_hint),
                actionLabel = stringResource(R.string.bluetooth_enable),
                action = {
                    val intent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
                    context.startActivity(intent)
                }
            )

        uiState.bleState.step == ConnectionStep.UNAVAILABLE ->
            BleScreenState.Requirement(
                title = stringResource(R.string.bluetooth_unavailable),
                description = stringResource(R.string.ble_unavailable_hint)
            )

        !permissionsGranted ->
            BleScreenState.Requirement(
                title = stringResource(R.string.permissions_required),
                description = stringResource(R.string.ble_permissions_hint)
            )

        !locationEnabled ->
            BleScreenState.Requirement(
                title = stringResource(R.string.location_button),
                description = stringResource(R.string.ble_location_hint),
                actionLabel = stringResource(R.string.location_button),
                action = {
                    val intent = Intent(Settings.ACTION_LOCATION_SOURCE_SETTINGS)
                    context.startActivity(intent)
                }
            )

        else -> BleScreenState.Ready()
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        snackbarHost = { SnackbarHost(snackbarHostState) }
    ) { innerPadding ->
        val contentModifier = Modifier.padding(innerPadding)

        when (screenState) {
            is BleScreenState.Requirement ->
                RequirementCard(
                    title = screenState.title,
                    description = screenState.description,
                    actionLabel = screenState.actionLabel,
                    onAction = screenState.action,
                    modifier = contentModifier
                )

            is BleScreenState.Ready ->
                BleContent(
                    uiState = uiState,
                    showSetupBanner = screenState.showSetupBanner,
                    onStartScan = viewModel::startScan,
                    onStopScan = viewModel::stopScan,
                    onConnect = viewModel::connect,
                    onDisconnect = viewModel::disconnect,
                    onLoadGame = viewModel::loadGame,
                    onFetchGames = viewModel::fetchGames,
                    onGameKeyChanged = viewModel::setSelectedGameKey,
                    modifier = contentModifier
                )
        }
    }
}

@Composable
private fun RequirementCard(
    title: String,
    description: String,
    actionLabel: String?,
    onAction: (() -> Unit)?,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        colors = CardDefaults.cardColors()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(text = title)
            Text(
                text = description,
                modifier = Modifier.padding(top = 8.dp)
            )
            if (actionLabel != null && onAction != null) {
                Button(
                    onClick = onAction,
                    modifier = Modifier.padding(top = 16.dp)
                ) {
                    Text(actionLabel)
                }
            }
        }
    }
}

@Composable
private fun BleContent(
    uiState: BleUiState,
    showSetupBanner: Boolean,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (me.aligator.e_chess.platform.ble.SimpleDevice) -> Unit,
    onDisconnect: () -> Unit,
    onLoadGame: (String) -> Unit,
    onFetchGames: () -> Unit,
    onGameKeyChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize()) {
        if (showSetupBanner) {
            SetupStatusBanner(
                isConnected = uiState.isConnected,
                isScanning = uiState.bleState.step == ConnectionStep.SCANNING
            )
        }
        ConnectionStatusCard(
            connectionState = uiState.bleState.connectedDevice,
            onDisconnect = onDisconnect
        )

        if (uiState.isConnected) {
            GameLoader(
                availableGames = uiState.availableGames,
                selectedGameKey = uiState.selectedGameKey,
                lastLoadedGame = uiState.lastLoadedGame,
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
                lastConnectedAddress = uiState.lastConnectedAddress,
                onStartScan = onStartScan,
                onStopScan = onStopScan,
                onConnect = onConnect
            )
        }
    }
}

@Composable
private fun SetupStatusBanner(
    isConnected: Boolean,
    isScanning: Boolean,
    modifier: Modifier = Modifier
) {
    val title = when {
        isConnected -> stringResource(R.string.setup_ready_title)
        isScanning -> stringResource(R.string.setup_scanning_title)
        else -> stringResource(R.string.setup_ready_to_scan_title)
    }
    val description = when {
        isConnected -> stringResource(R.string.setup_ready_description)
        isScanning -> stringResource(R.string.setup_scanning_description)
        else -> stringResource(R.string.setup_ready_to_scan_description)
    }
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        colors = CardDefaults.cardColors()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(text = title)
            Text(
                text = description,
                modifier = Modifier.padding(top = 6.dp)
            )
            val step = when {
                isConnected -> 3
                isScanning -> 2
                else -> 1
            }
            StepperRow(currentStep = step, modifier = Modifier.padding(top = 10.dp))
        }
    }
}

@Composable
private fun StepperRow(
    currentStep: Int,
    modifier: Modifier = Modifier
) {
    val steps = listOf(
        stringResource(R.string.setup_step_scan),
        stringResource(R.string.setup_step_connect),
        stringResource(R.string.setup_step_load)
    )
    androidx.compose.foundation.layout.Row(modifier = modifier) {
        steps.forEachIndexed { index, label ->
            val isCurrent = currentStep == index + 1
            Text(
                text = label,
                style = if (isCurrent) MaterialTheme.typography.labelMedium else MaterialTheme.typography.labelSmall,
                color = if (isCurrent) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
            )
            if (index < steps.lastIndex) {
                Text(
                    text = "  >  ",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
