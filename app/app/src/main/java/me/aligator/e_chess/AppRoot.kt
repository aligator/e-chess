package me.aligator.e_chess

import android.content.Intent
import android.content.res.Configuration
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.ui.BleScreen
import me.aligator.e_chess.ui.ConfigScreen
import me.aligator.e_chess.service.bluetooth.hasPermissions
import me.aligator.e_chess.service.bluetooth.requiredPermissions

private enum class AppDestination {
    BLE,
    CONFIG
}

@Composable
fun EChessApp() {
    val context = LocalContext.current
    val configStore = remember { ConfigurationStore(context.applicationContext) }

    var destination by rememberSaveable { mutableStateOf(AppDestination.BLE) }
    var language by rememberSaveable {
        mutableStateOf(AppLanguage.fromCode(configStore.getLanguage()))
    }
    var permissionsGranted by remember { mutableStateOf(hasPermissions(context)) }

    // Create launchers BEFORE CompositionLocalProvider
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { grantResults ->
        val permissions = requiredPermissions()
        val granted = permissions.all { permission ->
            grantResults[permission] == true
        }
        permissionsGranted = granted
    }

    // Request permissions on first launch
    LaunchedEffect(Unit) {
        if (!hasPermissions(context)) {
            permissionLauncher.launch(requiredPermissions().toTypedArray())
        }
    }

    val localizedContext = remember(language) {
        val config = Configuration(context.resources.configuration).apply {
            setLocale(language.locale)
        }
        context.createConfigurationContext(config)
    }

    Scaffold(
        bottomBar = {
            NavigationBar(modifier = Modifier.fillMaxWidth()) {
                NavigationBarItem(
                    selected = destination == AppDestination.BLE,
                    onClick = { destination = AppDestination.BLE },
                    icon = { },
                    label = { Text(stringResource(R.string.nav_bluetooth)) }
                )
                NavigationBarItem(
                    selected = destination == AppDestination.CONFIG,
                    onClick = { destination = AppDestination.CONFIG },
                    icon = { Icon(Icons.Default.Settings, contentDescription = null) },
                    label = { Text(stringResource(R.string.nav_settings)) }
                )
            }
        }
    ) { innerPadding ->
        CompositionLocalProvider(LocalContext provides localizedContext) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding)
            ) {
                when (destination) {
                    AppDestination.BLE -> BleScreen(
                        permissionsGranted = permissionsGranted
                    )

                    AppDestination.CONFIG -> ConfigScreen(
                        selectedLanguage = language,
                        onLanguageSelected = { newLanguage ->
                            language = newLanguage
                            configStore.saveLanguage(newLanguage.code)
                        }
                    )
                }
            }
        }
    }
}
