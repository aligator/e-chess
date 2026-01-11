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
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.model.AppError
import me.aligator.e_chess.repository.SettingsRepository
import me.aligator.e_chess.service.bluetooth.OtaState
import me.aligator.e_chess.service.bluetooth.OtaStatus
import java.io.InputStream

private const val LOG_TAG = "ConfigViewModel"

/**
 * ViewModel for Settings/Config screen.
 * Pure delegation to SettingsRepository.
 */
class ConfigViewModel(
    private val settingsRepository: SettingsRepository
) : ViewModel() {
    // Delegate to repository
    val lichessToken: StateFlow<String?> = settingsRepository.lichessToken
    val language: StateFlow<AppLanguage> = settingsRepository.language
    val otaState: StateFlow<OtaState> = settingsRepository.otaState
    val error: StateFlow<AppError?> = settingsRepository.error

    private val _otaUploadInProgress = MutableStateFlow(false)
    val otaUploadInProgress: StateFlow<Boolean> = _otaUploadInProgress.asStateFlow()

    init {
        // Observe OTA state changes to auto-reset UI on completion, error, or disconnect
        viewModelScope.launch {
            settingsRepository.otaState.collect { state ->
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

    fun saveLichessToken(token: String?) {
        settingsRepository.saveLichessToken(token)
    }

    fun saveLanguage(language: AppLanguage) {
        settingsRepository.saveLanguage(language)
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

                settingsRepository.uploadFirmware(firmwareData)
            } catch (e: Exception) {
                Log.e(LOG_TAG, "Failed to read firmware file", e)
                _otaUploadInProgress.value = false
            }
        }
    }

    fun resetOtaStatus() {
        settingsRepository.resetOtaStatus()
        _otaUploadInProgress.value = false
    }

    fun clearError() {
        settingsRepository.clearError()
    }
}
