package me.aligator.e_chess.feature.ble

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import me.aligator.e_chess.data.model.GameOption
import me.aligator.e_chess.platform.ble.BleState
import me.aligator.e_chess.platform.ble.ConnectionStep
import me.aligator.e_chess.platform.ble.ConnectedDevice
import me.aligator.e_chess.platform.ble.DeviceState
import me.aligator.e_chess.feature.ble.components.ConnectionStatusCard
import me.aligator.e_chess.feature.ble.components.DeviceScanner
import me.aligator.e_chess.feature.ble.components.GameLoader
import me.aligator.e_chess.ui.theme.EChessTheme

/**
 * Preview composables for BleScreen
 */
@Preview(showBackground = true)
@PreviewScreenSizes
@Composable
private fun BleScreenScanningPreview() {
    EChessTheme {
        BleScreenPreview(
            bleState = BleState(
                step = ConnectionStep.SCANNING,
                devices = emptyList()
            )
        )
    }
}

@Preview(showBackground = true)
@PreviewScreenSizes
@Composable
private fun BleScreenConnectedPreview() {
    EChessTheme {
        BleScreenPreview(
            bleState = BleState(
                step = ConnectionStep.IDLE,
                devices = emptyList(),
                connectedDevice = ConnectedDevice(
                    deviceState = DeviceState.CONNECTED,
                    address = "AA:BB:CC:DD:EE:FF",
                    characteristicsReady = true
                )
            )
        )
    }
}

@Composable
private fun BleScreenPreview(
    bleState: BleState,
    modifier: Modifier = Modifier
) {
    val uiState = BleUiState(
        bleState = bleState,
        devices = listOf(
            BleDeviceItem("Mock Board A", "00:11:22:33:44:55"),
            BleDeviceItem("Mock Board B", "AA:BB:CC:DD:EE:FF")
        ),
        availableGames = listOf(
            GameOption("abc123", "vs Magnus (abc123)"),
            GameOption("def456", "vs Hikaru (def456)")
        ),
        isConnected = bleState.connectedDevice.deviceState == DeviceState.CONNECTED && bleState.connectedDevice.characteristicsReady,
        selectedGameKey = ""
    )

    BleContentPreview(
        uiState = uiState,
        modifier = modifier
    )
}

@Composable
private fun BleContentPreview(
    uiState: BleUiState,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize()) {
        ConnectionStatusCard(connectionState = uiState.bleState.connectedDevice)

        if (uiState.isConnected) {
            GameLoader(
                availableGames = uiState.availableGames,
                selectedGameKey = uiState.selectedGameKey,
                onGameKeyChanged = {},
                onLoadGame = {},
                onFetchGames = {}
            )
        } else {
            DeviceScanner(
                scanning = uiState.bleState.step == ConnectionStep.SCANNING,
                devices = uiState.devices,
                connectedDevice = uiState.bleState.connectedDevice,
                onStartScan = {},
                onStopScan = {},
                onConnect = {}
            )
        }
    }
}
