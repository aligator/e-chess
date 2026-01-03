package me.aligator.e_chess.ui

import android.bluetooth.BluetoothDevice
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewScreenSizes
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.service.bluetooth.BleState
import me.aligator.e_chess.service.bluetooth.ConnectedDevice
import me.aligator.e_chess.service.bluetooth.ConnectionStep
import me.aligator.e_chess.service.bluetooth.DeviceState
import me.aligator.e_chess.service.bluetooth.SimpleDevice
import me.aligator.e_chess.ui.theme.EChessTheme

@Composable
fun BleScreenContent(
    bleState: BleState,
    permissionsGranted: Boolean,
    locationEnabled: Boolean,
    onRequestEnableBt: () -> Unit,
    onOpenLocationSettings: () -> Unit,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (SimpleDevice) -> Unit,
   // onLoadGame: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val textPadding = Modifier.padding(innerPadding)
        when {
            bleState.step == ConnectionStep.DISABLED ->
                    Button(onClick = onRequestEnableBt, modifier = textPadding) {
                        Text(stringResource(R.string.bluetooth_enable))
                    }
            bleState.step == ConnectionStep.UNAVAILABLE ->
                    Text(stringResource(R.string.bluetooth_unavailable), modifier = textPadding)
            permissionsGranted.not() ->
                    Text(stringResource(R.string.permissions_required), modifier = textPadding)
            locationEnabled.not() ->
                    Button(onClick = onOpenLocationSettings, modifier = textPadding) {
                        Text(stringResource(R.string.location_button))
                    }
            else ->
                    BleContent(
                            scanning = bleState.step == ConnectionStep.SCANNING,
                            connectionState = bleState.connectedDevice,
                            canLoadGame =  bleState.connectedDevice.deviceState == DeviceState.CONNECTED,
                            devices = bleState.devices,
                            onStartScan = onStartScan,
                            onStopScan = onStopScan,
                            onConnect = onConnect,
                        //    onLoadGame = onLoadGame,
                            modifier = textPadding
                    )
        }
    }
}

@Composable
private fun BleContent(
    scanning: Boolean,
    connectionState: ConnectedDevice,
    devices: List<SimpleDevice>,
    canLoadGame: Boolean,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (SimpleDevice) -> Unit,
  //  onLoadGame: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var gameKey by rememberSaveable { mutableStateOf("") }

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
            ) {
                Text(
                        if (scanning) stringResource(R.string.scan_stop)
                        else stringResource(R.string.scan_start)
                )
            }
            if (canLoadGame) {
                Spacer(modifier = Modifier.height(12.dp))
                OutlinedTextField(
                        value = gameKey,
                        onValueChange = { gameKey = it },
                        label = { Text(stringResource(R.string.game_key_label)) },
                        placeholder = { Text(stringResource(R.string.game_key_placeholder)) },
                        singleLine = true,
                        modifier = Modifier.padding(horizontal = 16.dp).fillMaxWidth()
                )
                Button(
                        onClick = {
                          //  onLoadGame(gameKey.trim())
                            gameKey = ""
                        },
                        enabled = gameKey.isNotBlank(),
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
                ) { Text(stringResource(R.string.load_game_button)) }
            }
        }
        items(devices) { device -> DeviceCard(device = device, onConnect = onConnect) }
    }
}

@Composable
private fun DeviceCard(
        device: SimpleDevice,
        onConnect: (SimpleDevice) -> Unit,
) {
    Card(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp).fillMaxWidth(),
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
        Button(onClick = { onConnect(device) }, modifier = Modifier.padding(16.dp)) {
            Text(stringResource(R.string.connect_button))
        }
    }
}

@Preview(showBackground = true)
@PreviewScreenSizes
@Composable
private fun BleScreenPreview() {
    EChessTheme {
        BleScreenContent(
                bleState =
                    BleState(
                                step = ConnectionStep.SCANNING,
                                devices = emptyList(),
                        ),
                permissionsGranted = true,
                locationEnabled = true,
                onRequestEnableBt = {},
                onOpenLocationSettings = {},
                onStartScan = {},
                onStopScan = {},
                onConnect = {},
              //  onLoadGame = {},
        )
    }
}
