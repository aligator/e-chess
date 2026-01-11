package me.aligator.e_chess.ui.components

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
import me.aligator.e_chess.service.bluetooth.BondingState
import me.aligator.e_chess.service.bluetooth.ConnectedDevice
import me.aligator.e_chess.service.bluetooth.DeviceState

@Composable
fun ConnectionStatusCard(
    connectionState: ConnectedDevice,
    onDisconnect: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer
        )
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row {
                Text(
                    text = "${stringResource(R.string.status_label)}: ${getConnectionStateText(connectionState)}",
                    style = MaterialTheme.typography.bodyLarge
                )
                if (connectionState.deviceState == DeviceState.CONNECTING || connectionState.bondingState == BondingState.BONDING) {
                    Spacer(modifier = Modifier.width(8.dp))
                    CircularProgressIndicator(
                        modifier = Modifier.size(20.dp),
                        strokeWidth = 2.dp
                    )
                }
            }

            if (connectionState.bondingState != BondingState.NONE && connectionState.bondingState != BondingState.BONDING) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "Pairing: ${getBondingStateText(connectionState.bondingState)}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = when (connectionState.bondingState) {
                        BondingState.BONDED -> MaterialTheme.colorScheme.primary
                        BondingState.FAILED -> MaterialTheme.colorScheme.error
                        else -> MaterialTheme.colorScheme.onSurface
                    }
                )
            }

            if (connectionState.deviceState == DeviceState.CONNECTED && connectionState.characteristicsReady) {
                Spacer(modifier = Modifier.height(8.dp))
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

@Composable
private fun getConnectionStateText(state: ConnectedDevice): String {
    return when (state.deviceState) {
        DeviceState.CONNECTED -> if (state.characteristicsReady) "Connected" else "Connecting"
        DeviceState.CONNECTING -> "Connecting"
        DeviceState.DISCONNECTED -> "Disconnected"
        DeviceState.DISCONNECTING -> "Disconnecting"
        DeviceState.UNKNOWN -> "Unknown"
    }
}

@Composable
private fun getBondingStateText(state: BondingState): String {
    return when (state) {
        BondingState.NONE -> "Not Paired"
        BondingState.BONDING -> "Pairing..."
        BondingState.BONDED -> "Paired"
        BondingState.FAILED -> "Failed"
    }
}
