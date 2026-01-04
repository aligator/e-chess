package me.aligator.e_chess.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.R
import me.aligator.e_chess.service.bluetooth.SimpleDevice

@Composable
fun DeviceScanner(
    scanning: Boolean,
    devices: List<SimpleDevice>,
    onStartScan: () -> Unit,
    onStopScan: () -> Unit,
    onConnect: (SimpleDevice) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier) {
        Button(
            onClick = if (scanning) onStopScan else onStartScan,
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp)
        ) {
            Text(
                if (scanning) stringResource(R.string.scan_stop)
                else stringResource(R.string.scan_start)
            )
        }

        if (devices.isNotEmpty()) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = stringResource(R.string.available_devices),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
        }

        devices.forEach { device ->
            DeviceCard(
                device = device,
                onConnect = onConnect
            )
        }
    }
}

@Composable
private fun DeviceCard(
    device: SimpleDevice,
    onConnect: (SimpleDevice) -> Unit,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp),
        colors = CardDefaults.cardColors()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = device.name ?: stringResource(R.string.unknown_device),
                style = MaterialTheme.typography.titleMedium
            )
            Text(
                text = device.address,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(top = 4.dp)
            )
            Button(
                onClick = { onConnect(device) },
                modifier = Modifier.padding(top = 8.dp)
            ) {
                Text(stringResource(R.string.connect_button))
            }
        }
    }
}
