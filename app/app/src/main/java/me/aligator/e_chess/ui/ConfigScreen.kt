package me.aligator.e_chess.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.R
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.service.bluetooth.OtaAction
import me.aligator.e_chess.service.bluetooth.OtaStatus
import me.aligator.e_chess.ui.theme.EChessTheme

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        else -> String.format("%.1f MB", bytes / (1024.0 * 1024.0))
    }
}

@Composable
fun ConfigScreen(
    selectedLanguage: AppLanguage,
    onLanguageSelected: (AppLanguage) -> Unit,
    modifier: Modifier = Modifier,
    otaAction: OtaAction? = null,
    onOtaSelectFile: (() -> Unit)? = null,
    otaFileUri: Uri? = null,
    onOtaFileConsumed: () -> Unit = {},
) {
    val context = LocalContext.current
    val configStore = remember { ConfigurationStore(context.applicationContext) }
    val viewModel: ConfigViewModel = viewModel()

    var token by rememberSaveable { mutableStateOf("") }
    var savedMessage by remember { mutableStateOf("") }

    LaunchedEffect(configStore) {
        configStore.getLichessToken()?.let { token = it }
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
        Text(text = stringResource(R.string.config_title))
        LanguageSelector(
            selectedLanguage = selectedLanguage,
            onLanguageSelected = onLanguageSelected,
            modifier = Modifier.padding(top = 8.dp, bottom = 16.dp)
        )
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
                configStore.saveLichessToken(token)
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

        Spacer(modifier = Modifier.height(32.dp))
        HorizontalDivider()
        Spacer(modifier = Modifier.height(16.dp))

        if (otaAction != null && onOtaSelectFile != null) {
            OtaSection(
                viewModel = viewModel,
                otaAction = otaAction,
                onSelectFileClick = onOtaSelectFile
            )
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun ConfigScreenPreview() {
    EChessTheme {
        ConfigScreen(
            selectedLanguage = AppLanguage.DE,
            onLanguageSelected = {}
        )
    }
}

@Composable
private fun OtaSection(
    viewModel: ConfigViewModel,
    otaAction: OtaAction,
    onSelectFileClick: () -> Unit
) {
    val otaState by otaAction.otaState.collectAsState()
    val uploadInProgress by viewModel.otaUploadInProgress.collectAsState()

    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = stringResource(R.string.ota_title),
            style = MaterialTheme.typography.titleMedium
        )

        Spacer(modifier = Modifier.height(16.dp))

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
                Button(
                    onClick = onSelectFileClick
                ) {
                    Text(text = stringResource(R.string.ota_select_file))
                }
            }
        }
    }
}

@Composable
private fun LanguageSelector(
    selectedLanguage: AppLanguage,
    onLanguageSelected: (AppLanguage) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Text(text = stringResource(R.string.language_label))
        Row(
            modifier = Modifier.padding(top = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            AppLanguage.values().forEach { lang ->
                Button(
                    onClick = { onLanguageSelected(lang) },
                    enabled = lang != selectedLanguage
                ) {
                    Text(text = "${lang.flag} ${lang.name}")
                }
            }
        }
    }
}
