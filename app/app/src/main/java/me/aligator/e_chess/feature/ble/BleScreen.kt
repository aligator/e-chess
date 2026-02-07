package me.aligator.e_chess.feature.ble

import android.bluetooth.BluetoothAdapter
import android.content.Context
import android.content.Intent
import android.location.LocationManager
import android.provider.Settings
import android.util.Log
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
import me.aligator.e_chess.ui.AppTopBar
import me.aligator.e_chess.ui.UiEvent
import me.aligator.e_chess.ui.UiMessage

private fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

private fun Context.safeStartActivity(intent: Intent) {
    try {
        startActivity(intent)
    } catch (e: SecurityException) {
        Log.w("BleScreen", "startActivity blocked by missing permission", e)
    }
}

private sealed class BleScreenState {
    data class Requirement(
        val title: String,
        val description: String,
        val actionLabel: String? = null,
        val action: (() -> Unit)? = null,
    ) : BleScreenState()

    data object Ready : BleScreenState()
}

@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
fun BleScreen(
    permissionsGranted: Boolean,
    modifier: Modifier = Modifier,
    bluetoothService: BoardBleService? = null,
    mockModeEnabled: Boolean = false,
    onOpenGame: () -> Unit,
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

    LaunchedEffect(mockModeEnabled) {
        viewModel.setMockMode(mockModeEnabled)
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
        mockModeEnabled -> BleScreenState.Ready
        uiState.bleState.step == ConnectionStep.DISABLED ->
            BleScreenState.Requirement(
                title = stringResource(R.string.bluetooth_enable),
                description = stringResource(R.string.ble_bluetooth_hint),
                actionLabel = stringResource(R.string.bluetooth_enable),
                action = {
                    val intent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
                    context.safeStartActivity(intent)
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
                    context.safeStartActivity(intent)
                }
            )

        else -> BleScreenState.Ready
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            AppTopBar(title = stringResource(R.string.nav_chess))
        }
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

            BleScreenState.Ready ->
                BleContent(
                    uiState = uiState,
                    onStartScan = viewModel::startScan,
                    onStopScan = viewModel::stopScan,
                    onConnect = viewModel::connect,
                    onDisconnect = viewModel::disconnect,
                    onOpenGame = onOpenGame,
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
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (BleDeviceItem) -> Unit,
    onDisconnect: () -> Unit,
    onOpenGame: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize()) {
        ConnectionStatusCard(
            connectionState = uiState.bleState.connectedDevice,
            onDisconnect = onDisconnect
        )

        if (uiState.isConnected) {
            Button(
                onClick = onOpenGame,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
            ) {
                Text(stringResource(R.string.select_game))
            }
        } else {
            DeviceScanner(
                scanning = uiState.bleState.step == ConnectionStep.SCANNING,
                devices = uiState.devices,
                connectedDevice = uiState.bleState.connectedDevice,
                onStartScan = onStartScan,
                onStopScan = onStopScan,
                onConnect = onConnect
            )
        }
    }
}
