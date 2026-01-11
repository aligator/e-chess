package me.aligator.e_chess.ui

import android.net.Uri
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.R
import me.aligator.e_chess.service.bluetooth.OtaStatus
import me.aligator.e_chess.ui.theme.EChessTheme
import org.koin.androidx.compose.koinViewModel

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        else -> String.format("%.1f MB", bytes / (1024.0 * 1024.0))
    }
}

@Composable
fun ConfigScreen(
    modifier: Modifier = Modifier,
    onOtaSelectFile: (() -> Unit)? = null,
    otaFileUri: Uri? = null,
    onOtaFileConsumed: () -> Unit = {},
    viewModel: ConfigViewModel = koinViewModel(),
    bleViewModel: BleViewModel = koinViewModel()
) {
    val context = LocalContext.current
    val snackbarHostState = remember { SnackbarHostState() }

    // Collect state from ViewModel
    val lichessToken by viewModel.lichessToken.collectAsState()
    val language by viewModel.language.collectAsState()
    val error by viewModel.error.collectAsState()
    val bleUiState by bleViewModel.uiState.collectAsState()

    var token by rememberSaveable { mutableStateOf("") }
    var savedMessage by remember { mutableStateOf("") }

    // Initialize token from repository
    LaunchedEffect(lichessToken) {
        lichessToken?.let { token = it }
    }

    // Show error in Snackbar
    LaunchedEffect(error) {
        error?.let {
            snackbarHostState.showSnackbar(it.message)
            viewModel.clearError()
        }
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
        SnackbarHost(hostState = snackbarHostState)

        LanguageSelector(
            selectedLanguage = language,
            onLanguageSelected = { viewModel.saveLanguage(it) },
            modifier = Modifier.padding(bottom = 16.dp)
        )

        HorizontalDivider()
        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = stringResource(R.string.config_title),
            style = MaterialTheme.typography.titleMedium
        )

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
                viewModel.saveLichessToken(token)
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

        if (onOtaSelectFile != null) {
            OtaSection(
                viewModel = viewModel,
                onSelectFileClick = onOtaSelectFile,
                isDeviceConnected = bleUiState.isConnected
            )
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun ConfigScreenPreview() {
    EChessTheme {
        ConfigScreen()
    }
}

@Composable
private fun OtaSection(
    viewModel: ConfigViewModel,
    onSelectFileClick: () -> Unit,
    isDeviceConnected: Boolean
) {
    val otaState by viewModel.otaState.collectAsState()
    val uploadInProgress by viewModel.otaUploadInProgress.collectAsState()

    // Auto-reset UI after 3 seconds on completion or error
    LaunchedEffect(otaState.status) {
        if (otaState.status == OtaStatus.COMPLETED || otaState.status == OtaStatus.ERROR) {
            kotlinx.coroutines.delay(3000)
            viewModel.resetOtaStatus()
        }
    }

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
        Text(
            text = stringResource(R.string.language_label),
            style = MaterialTheme.typography.titleMedium
        )

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
