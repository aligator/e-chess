package me.aligator.e_chess.ui

import android.bluetooth.BluetoothDevice
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.service.BleUiState
import me.aligator.e_chess.service.SimpleDevice
import me.aligator.e_chess.ui.theme.EChessTheme

@Composable
fun BleScreenContent(
    uiState: BleUiState,
    permissionsGranted: Boolean,
    locationEnabled: Boolean,
    bluetoothServiceConnected: Boolean,
    onRequestEnableBt: () -> Unit,
    onOpenLocationSettings: () -> Unit,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (BluetoothDevice) -> Unit,
    modifier: Modifier = Modifier,
) {
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val textPadding = Modifier.padding(innerPadding)
        when {
            uiState.connectionState == "Bluetooth deaktiviert" -> Button(
                onClick = onRequestEnableBt,
                modifier = textPadding
            ) { Text(stringResource(R.string.bluetooth_enable)) }

            uiState.connectionState == "Bluetooth nicht verfügbar" -> Text(
                stringResource(R.string.bluetooth_unavailable),
                modifier = textPadding
            )

            permissionsGranted.not() -> Text(stringResource(R.string.permissions_required), modifier = textPadding)
            locationEnabled.not() -> Button(
                onClick = onOpenLocationSettings,
                modifier = textPadding
            ) { Text(stringResource(R.string.location_button)) }

            bluetoothServiceConnected.not() -> Text(
                stringResource(R.string.service_connecting),
                modifier = textPadding
            )

            else -> BleContent(
                scanning = uiState.scanning,
                connectionState = uiState.connectionState,
                devices = uiState.devices,
                onStartScan = onStartScan,
                onStopScan = onStopScan,
                onConnect = onConnect,
                modifier = textPadding
            )
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
                text = "${stringResource(R.string.status_label)}: $connectionState",
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.padding(16.dp)
            )
            Button(
                onClick = if (scanning) onStopScan else onStartScan,
                modifier = Modifier.padding(horizontal = 16.dp)
            ) { Text(if (scanning) stringResource(R.string.scan_stop) else stringResource(R.string.scan_start)) }
        }
        items(devices) { device ->
            DeviceCard(device = device, onConnect = onConnect)
        }
    }
}

@Composable
private fun DeviceCard(
    device: SimpleDevice,
    onConnect: (BluetoothDevice) -> Unit,
) {
    Card(
        modifier = Modifier
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .fillMaxWidth(),
        colors = CardDefaults.cardColors()
    ) {
        Text(
            text = device.name ?: stringResource(R.string.unknown_device),
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
        ) { Text(stringResource(R.string.connect_button)) }
    }
}

@Preview(showBackground = true)
@PreviewScreenSizes
@Composable
private fun BleScreenPreview() {
    EChessTheme {
        BleScreenContent(
            uiState = BleUiState(
                scanning = false,
                connectionState = "Verbunden",
                devices = emptyList()
            ),
            permissionsGranted = true,
            locationEnabled = true,
            bluetoothServiceConnected = true,
            onRequestEnableBt = {},
            onOpenLocationSettings = {},
            onStartScan = {},
            onStopScan = {},
            onConnect = {},
        )
    }
}
