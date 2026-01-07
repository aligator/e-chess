package me.aligator.e_chess.ui

import android.content.Context
import android.net.Uri
import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.service.bluetooth.OtaAction
import java.io.InputStream

private const val LOG_TAG = "ConfigViewModel"

class ConfigViewModel : ViewModel() {
    private var otaAction: OtaAction? = null

    private val _otaUploadInProgress = MutableStateFlow(false)
    val otaUploadInProgress: StateFlow<Boolean> = _otaUploadInProgress.asStateFlow()

    fun setOtaAction(action: OtaAction) {
        otaAction = action

        // Observe OTA state changes to auto-reset UI on completion, error, or disconnect
        viewModelScope.launch {
            action.otaState.collect { state ->
                when (state.status) {
                    me.aligator.e_chess.service.bluetooth.OtaStatus.IDLE,
                    me.aligator.e_chess.service.bluetooth.OtaStatus.COMPLETED,
                    me.aligator.e_chess.service.bluetooth.OtaStatus.ERROR -> {
                        _otaUploadInProgress.value = false
                    }

                    me.aligator.e_chess.service.bluetooth.OtaStatus.UPLOADING -> {
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
