package me.aligator.e_chess.feature.ble.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.platform.ble.ConnectedDevice
import me.aligator.e_chess.platform.ble.DeviceState

@Composable
fun ConnectionStatusCard(
    connectionState: ConnectedDevice,
    onDisconnect: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    val statusText = when (connectionState.deviceState) {
        DeviceState.CONNECTED -> stringResource(
            if (connectionState.characteristicsReady) R.string.ble_status_connected
            else R.string.ble_status_preparing
        )
        DeviceState.CONNECTING -> stringResource(R.string.ble_status_connecting)
        DeviceState.DISCONNECTING -> stringResource(R.string.ble_status_disconnecting)
        DeviceState.DISCONNECTED -> stringResource(R.string.ble_status_disconnected)
        DeviceState.UNKNOWN -> stringResource(R.string.ble_status_unknown)
    }

    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    text = "${stringResource(R.string.status_label)}: $statusText",
                    style = MaterialTheme.typography.bodyLarge
                )
                if (connectionState.deviceState == DeviceState.CONNECTING) {
                    Spacer(modifier = Modifier.width(8.dp))
                    CircularProgressIndicator(
                        modifier = Modifier.size(20.dp),
                        strokeWidth = 2.dp
                    )
                }
            }

            if (connectionState.deviceState == DeviceState.CONNECTED && connectionState.characteristicsReady) {
                Spacer(modifier = Modifier.height(8.dp))
                if (connectionState.address != null) {
                    Text(
                        text = stringResource(R.string.ble_connected_address, connectionState.address),
                        style = MaterialTheme.typography.bodySmall
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                }
                Button(
                    onClick = onDisconnect,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(stringResource(R.string.disconnect_button))
                }
            }
        }
    }
}
