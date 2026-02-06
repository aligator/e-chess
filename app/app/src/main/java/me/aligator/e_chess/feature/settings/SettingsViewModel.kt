package me.aligator.e_chess.feature.settings

import android.content.Context
import android.net.Uri
import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.platform.ble.BoardOtaAction
import me.aligator.e_chess.platform.ble.OtaStatus
import java.io.InputStream

private const val LOG_TAG = "SettingsViewModel"

class SettingsViewModel : ViewModel() {
    private var otaAction: BoardOtaAction? = null

    private val _otaUploadInProgress = MutableStateFlow(false)
    val otaUploadInProgress: StateFlow<Boolean> = _otaUploadInProgress.asStateFlow()

    fun setOtaAction(action: BoardOtaAction) {
        otaAction = action

        // Observe OTA state changes to auto-reset UI on completion, error, or disconnect
        viewModelScope.launch {
            action.otaState.collect { state ->
                when (state.status) {
                    OtaStatus.IDLE,
                    OtaStatus.COMPLETED,
                    OtaStatus.ERROR -> {
                        _otaUploadInProgress.value = false
                    }

                    OtaStatus.UPLOADING -> {
                        // Keep upload in progress
                    }
                }
            }
        }
    }

    fun uploadFirmware(context: Context, uri: Uri) {
        viewModelScope.launch {
            try {
                _otaUploadInProgress.value = true

                val inputStream: InputStream? = context.contentResolver.openInputStream(uri)
                if (inputStream == null) {
                    Log.e(LOG_TAG, "Failed to open input stream for URI: $uri")
                    _otaUploadInProgress.value = false
                    return@launch
                }

                // Read raw firmware data
                val firmwareData = inputStream.readBytes()
                inputStream.close()

                Log.d(LOG_TAG, "Uploading firmware: ${firmwareData.size} bytes")

                otaAction?.uploadFirmware(firmwareData, firmwareData.size.toLong())
            } catch (e: Exception) {
                Log.e(LOG_TAG, "Failed to read firmware file", e)
                _otaUploadInProgress.value = false
            }
        }
    }

    fun resetOtaStatus() {
        otaAction?.resetStatus()
        _otaUploadInProgress.value = false
    }
}
