package me.aligator.e_chess

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.net.Uri
import android.os.IBinder
import android.provider.Settings
import android.util.Log
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.SaverScope
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.res.stringResource
import me.aligator.e_chess.ui.BleScreen
import me.aligator.e_chess.ui.ConfigScreen
import me.aligator.e_chess.ui.ConfigViewModel
import me.aligator.e_chess.service.bluetooth.BluetoothService
import org.koin.androidx.compose.koinViewModel
import me.aligator.e_chess.service.bluetooth.hasPermissions
import me.aligator.e_chess.service.bluetooth.requiredPermissions

private enum class AppDestination {
    BLE,
    CONFIG
}

@Composable
fun EChessApp() {
    val context = LocalContext.current
    val isPreview = LocalInspectionMode.current
    val configViewModel: ConfigViewModel = koinViewModel()

    var destination by rememberSaveable { mutableStateOf(AppDestination.BLE) }
    var permissionsGranted by remember { mutableStateOf(hasPermissions(context)) }
    val language by configViewModel.language.collectAsState()

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

    // Bind to BluetoothService
    DisposableEffect(Unit) {
        if (isPreview) return@DisposableEffect onDispose {}

        val connection = object : android.content.ServiceConnection {
            override fun onServiceConnected(name: ComponentName?, binder: IBinder?) {
                // BluetoothService is now managed via Koin repositories
            }

            override fun onServiceDisconnected(name: ComponentName?) {
                // BluetoothService is now managed via Koin repositories
            }
        }

        val intent = Intent(context, BluetoothService::class.java)
        context.startService(intent)
        val bound = context.bindService(intent, connection, Context.BIND_AUTO_CREATE)

        if (!bound) {
            Log.e("AppRoot", "Failed to bind BluetoothService")
        }

        onDispose {
            if (bound) context.unbindService(connection)
        }
    }

    // File picker for OTA - must be created BEFORE CompositionLocalProvider
    // Use rememberSaveable with custom Saver to survive configuration changes (rotation)
    var otaFileUri by rememberSaveable(
        stateSaver = Saver<Uri?, String>(
            save = { it?.toString() },
            restore = { Uri.parse(it) }
        )
    ) { mutableStateOf(null) }

    val otaFilePicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        otaFileUri = uri
    }

    val localizedContext = remember(language) {
        val config = Configuration(context.resources.configuration).apply {
            setLocale(language.locale)
        }
        context.createConfigurationContext(config)
    }

    CompositionLocalProvider(LocalContext provides localizedContext) {
        Scaffold(
            bottomBar = {
                NavigationBar(modifier = Modifier.fillMaxWidth()) {
                    NavigationBarItem(
                        selected = destination == AppDestination.BLE,
                        onClick = { destination = AppDestination.BLE },
                        icon = {
                            Text(
                                text = "♟",
                                fontSize = 24.sp,
                                fontWeight = FontWeight.Normal
                            )
                        },
                        label = { Text(stringResource(R.string.nav_chess)) }
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
                        onOtaSelectFile = { otaFilePicker.launch("*/*") },
                        otaFileUri = otaFileUri,
                        onOtaFileConsumed = { otaFileUri = null }
                    )
                }
            }
        }
    }
}
