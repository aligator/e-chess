package me.aligator.e_chess.ui

import android.content.Context
import android.location.LocationManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable

fun isLocationEnabled(context: Context): Boolean {
    val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
    return locationManager?.isProviderEnabled(LocationManager.GPS_PROVIDER) == true ||
            locationManager?.isProviderEnabled(LocationManager.NETWORK_PROVIDER) == true
}

@Composable
fun rememberPermissionLauncher(
    requiredPermissions: List<String>,
    onResult: (Boolean) -> Unit,
): () -> Unit {
    val launcher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { grantResults ->
        val granted = requiredPermissions.all { permission ->
            grantResults[permission] == true
        }
        onResult(granted)
    }
    return { launcher.launch(requiredPermissions.toTypedArray()) }
}
