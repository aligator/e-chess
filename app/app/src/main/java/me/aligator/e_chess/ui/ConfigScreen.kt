package me.aligator.e_chess.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.R
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.ui.theme.EChessTheme

@Composable
fun ConfigScreen(
    selectedLanguage: AppLanguage,
    onLanguageSelected: (AppLanguage) -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val configStore = remember { ConfigurationStore(context.applicationContext) }

    var token by rememberSaveable { mutableStateOf("") }
    var savedMessage by remember { mutableStateOf("") }

    LaunchedEffect(configStore) {
        configStore.getLichessToken()?.let { token = it }
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
