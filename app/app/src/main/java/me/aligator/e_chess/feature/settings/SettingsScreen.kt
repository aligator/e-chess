package me.aligator.e_chess.feature.settings

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.R
import me.aligator.e_chess.data.SettingsStore
import me.aligator.e_chess.data.DebugLogStore
import me.aligator.e_chess.platform.ble.BoardBleService
import me.aligator.e_chess.platform.ble.BoardOtaAction
import me.aligator.e_chess.platform.ble.BleState
import me.aligator.e_chess.platform.ble.DeviceState
import me.aligator.e_chess.platform.ble.OtaStatus
import me.aligator.e_chess.ui.theme.EChessTheme

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        else -> String.format("%.1f MB", bytes / (1024.0 * 1024.0))
    }
}

private data class DebugLogUiState(
    val fileName: String?,
    val fileSizeBytes: Long
)

@Composable
fun SettingsScreen(
    selectedLanguage: AppLanguage,
    onLanguageSelected: (AppLanguage) -> Unit,
    modifier: Modifier = Modifier,
    otaAction: BoardOtaAction? = null,
    bleService: BoardBleService? = null,
    onOtaSelectFile: (() -> Unit)? = null,
    otaFileUri: Uri? = null,
    onOtaFileConsumed: () -> Unit = {},
) {
    val context = LocalContext.current
    val coroutineScope = rememberCoroutineScope()
    val settingsStore = remember { SettingsStore(context.applicationContext) }
    val viewModel: SettingsViewModel = viewModel()

    var token by rememberSaveable { mutableStateOf("") }
    var savedMessage by remember { mutableStateOf("") }
    var debugLoggingEnabled by remember { mutableStateOf(false) }

    LaunchedEffect(settingsStore) {
        settingsStore.getLichessToken()?.let { token = it }
    }

    LaunchedEffect(otaAction) {
        otaAction?.let { viewModel.setOtaAction(it) }
    }

    // Process the selected OTA file
    LaunchedEffect(otaFileUri) {
        if (otaFileUri != null) {
            viewModel.uploadFirmware(context, otaFileUri)
            onOtaFileConsumed()
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp)
    ) {
        SectionCard(
            title = stringResource(R.string.language_label),
            description = stringResource(R.string.language_description)
        ) {
            LanguageSelector(
                selectedLanguage = selectedLanguage,
                onLanguageSelected = onLanguageSelected,
                modifier = Modifier.padding(top = 8.dp)
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        SectionCard(
            title = stringResource(R.string.settings_token_title),
            description = stringResource(R.string.token_description)
        ) {
            TokenLink()

            OutlinedTextField(
                value = token,
                onValueChange = { token = it },
                label = { Text(stringResource(R.string.token_label)) },
                singleLine = true,
                modifier = Modifier
                    .padding(top = 8.dp)
                    .fillMaxWidth()
            )
            Button(
                onClick = {
                    settingsStore.saveLichessToken(token)
                    savedMessage = context.getString(R.string.token_saved)
                },
                modifier = Modifier.padding(top = 12.dp)
            ) { Text(stringResource(R.string.save_token)) }

            if (savedMessage.isNotEmpty()) {
                Text(
                    text = savedMessage,
                    modifier = Modifier.padding(top = 8.dp)
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        SectionCard(
            title = stringResource(R.string.debug_logs_title),
            description = stringResource(R.string.debug_logs_description)
        ) {
            DebugLogSection(
                enabled = debugLoggingEnabled,
                onToggle = { enabled ->
                    debugLoggingEnabled = enabled
                    if (enabled) {
                        DebugLogStore.start(context)
                    } else {
                        coroutineScope.launch {
                            DebugLogStore.stopAndAwait()
                            val uri = DebugLogStore.shareUri(context) ?: return@launch
                            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                                type = "application/octet-stream"
                                putExtra(Intent.EXTRA_STREAM, uri)
                                clipData = ClipData.newRawUri("debug-log", uri)
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            }
                            val chooserIntent = Intent.createChooser(
                                shareIntent,
                                context.getString(R.string.debug_logs_share_title)
                            ).apply {
                                if (context !is Activity) {
                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                }
                            }
                            context.startActivity(chooserIntent)
                        }
                    }
                }
            )
        }

        if (otaAction != null && onOtaSelectFile != null) {
            Spacer(modifier = Modifier.height(16.dp))
            SectionCard(
                title = stringResource(R.string.ota_title),
                description = stringResource(R.string.ota_description)
            ) {
                OtaSection(
                    viewModel = viewModel,
                    otaAction = otaAction,
                    bleService = bleService,
                    onSelectFileClick = onOtaSelectFile
                )
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun SettingsScreenPreview() {
    EChessTheme {
        SettingsScreen(
            selectedLanguage = AppLanguage.DE,
            onLanguageSelected = {}
        )
    }
}

@Composable
private fun DebugLogSection(
    enabled: Boolean,
    onToggle: (Boolean) -> Unit
) {
    val logUiState by produceState(
        initialValue = DebugLogUiState(
            fileName = DebugLogStore.currentLogFileName(),
            fileSizeBytes = DebugLogStore.currentLogFileSizeBytes() ?: 0L
        ),
        key1 = enabled
    ) {
        while (true) {
            value = DebugLogUiState(
                fileName = DebugLogStore.currentLogFileName(),
                fileSizeBytes = DebugLogStore.currentLogFileSizeBytes() ?: 0L
            )
            delay(1000)
        }
    }
    val logAvailable = logUiState.fileSizeBytes > 0L

    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                text = stringResource(R.string.debug_logs_toggle),
                modifier = Modifier.weight(1f)
            )
            Switch(
                checked = enabled,
                onCheckedChange = onToggle
            )
        }

        Text(
            text = stringResource(
                R.string.debug_logs_file_size,
                formatBytes(logUiState.fileSizeBytes)
            ),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 6.dp)
        )

        val logFileName = logUiState.fileName
        if (logFileName != null) {
            Text(
                text = stringResource(R.string.debug_logs_file_name, logFileName),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 2.dp)
            )
        }

        if (enabled && !logAvailable) {
            Text(
                text = stringResource(R.string.debug_logs_no_file),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp)
            )
        }
    }
}

@Composable
private fun OtaSection(
    viewModel: SettingsViewModel,
    otaAction: BoardOtaAction,
    bleService: BoardBleService?,
    onSelectFileClick: () -> Unit
) {
    val otaState by otaAction.otaState.collectAsState()
    val uploadInProgress by viewModel.otaUploadInProgress.collectAsState()
    val bleState by (bleService?.ble?.bleState
        ?: MutableStateFlow(BleState())).collectAsState()

    val isDeviceConnected =
        bleState.connectedDevice.deviceState == DeviceState.CONNECTED

    // Auto-reset UI after 3 seconds on completion or error
    LaunchedEffect(otaState.status) {
        if (otaState.status == OtaStatus.COMPLETED || otaState.status == OtaStatus.ERROR) {
            kotlinx.coroutines.delay(3000)
            viewModel.resetOtaStatus()
        }
    }

    Column(modifier = Modifier.fillMaxWidth()) {
        when {
            otaState.status == OtaStatus.UPLOADING || uploadInProgress -> {
                Column(
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = stringResource(R.string.ota_uploading),
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(bottom = 8.dp)
                    )

                    LinearProgressIndicator(
                        progress = { otaState.progress },
                        modifier = Modifier.fillMaxWidth(),
                    )

                    if (otaState.totalBytes > 0) {
                        Text(
                            text = "${formatBytes(otaState.bytesUploaded)} / ${formatBytes(otaState.totalBytes)} (${(otaState.progress * 100).toInt()}%)",
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.padding(top = 4.dp)
                        )
                    }
                }
            }

            otaState.status == OtaStatus.COMPLETED -> {
                Text(
                    text = stringResource(R.string.ota_completed),
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
                Button(
                    onClick = { viewModel.resetOtaStatus() }
                ) {
                    Text(text = "OK")
                }
            }

            otaState.status == OtaStatus.ERROR -> {
                Text(
                    text = stringResource(R.string.ota_error, otaState.errorMessage ?: "Unknown"),
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
                Button(
                    onClick = { viewModel.resetOtaStatus() }
                ) {
                    Text(text = "OK")
                }
            }

            else -> {
                Column(modifier = Modifier.fillMaxWidth()) {
                    Button(
                        onClick = onSelectFileClick,
                        enabled = isDeviceConnected
                    ) {
                        Text(text = stringResource(R.string.ota_select_file))
                    }

                    if (!isDeviceConnected) {
                        Text(
                            text = stringResource(R.string.ota_device_not_connected),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 8.dp)
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun TokenLink(
    modifier: Modifier = Modifier
) {
    val uriHandler = LocalUriHandler.current
    val url =
        "https://lichess.org/account/oauth/token/create?scopes[]=follow:read&scopes[]=challenge:read&scopes[]=challenge:write&scopes[]=board:play&description=EChess+Board+Token"

    TextButton(
        onClick = { uriHandler.openUri(url) },
        modifier = modifier.padding(top = 4.dp)
    ) {
        Text(
            text = stringResource(R.string.create_token_link),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.primary,
            textDecoration = TextDecoration.Underline
        )
    }
}

@Composable
private fun LanguageSelector(
    selectedLanguage: AppLanguage,
    onLanguageSelected: (AppLanguage) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }

    Column(modifier = modifier.fillMaxWidth()) {
        OutlinedButton(
            onClick = { expanded = true },
            modifier = Modifier.padding(top = 8.dp)
        ) {
            Text(text = "${selectedLanguage.flag} ${selectedLanguage.name}")
            Spacer(modifier = Modifier.width(8.dp))
            Icon(
                imageVector = Icons.Default.ArrowDropDown,
                contentDescription = null
            )
        }

        DropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            AppLanguage.values().forEach { lang ->
                DropdownMenuItem(
                    text = { Text("${lang.flag} ${lang.name}") },
                    onClick = {
                        onLanguageSelected(lang)
                        expanded = false
                    }
                )
            }
        }
    }
}

@Composable
private fun SectionCard(
    title: String,
    description: String,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium
            )
            Text(
                text = description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp)
            )
            content()
        }
    }
}
