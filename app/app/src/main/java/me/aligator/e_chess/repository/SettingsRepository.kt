package me.aligator.e_chess.repository

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.AppLanguage
import me.aligator.e_chess.model.AppError
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.service.bluetooth.OtaAction
import me.aligator.e_chess.service.bluetooth.OtaState

/**
 * Repository for app settings and OTA firmware updates.
 * Single Source of Truth for:
 * - Lichess token
 * - Language preference
 * - OTA state
 */
class SettingsRepository(
    private val configStore: ConfigurationStore
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    // Lichess Token
    private val _lichessToken = MutableStateFlow<String?>(null)
    val lichessToken: StateFlow<String?> = _lichessToken.asStateFlow()

    // Language
    private val _language = MutableStateFlow<AppLanguage>(AppLanguage.SYSTEM)
    val language: StateFlow<AppLanguage> = _language.asStateFlow()

    // OTA State
    private val _otaState = MutableStateFlow(OtaState())
    val otaState: StateFlow<OtaState> = _otaState.asStateFlow()

    // Error
    private val _error = MutableStateFlow<AppError?>(null)
    val error: StateFlow<AppError?> = _error.asStateFlow()

    // OTA Action reference
    private var otaAction: OtaAction? = null

    init {
        loadInitialSettings()
    }

    private fun loadInitialSettings() {
        // Load lichess token
        _lichessToken.value = configStore.getLichessToken()

        // Load language
        val languageCode = configStore.getLanguage()
        _language.value = AppLanguage.fromCode(languageCode)
    }

    /**
     * Save Lichess API token
     */
    fun saveLichessToken(token: String?) {
        try {
            configStore.saveLichessToken(token)
            _lichessToken.value = token
            _error.value = null
        } catch (e: Exception) {
            _error.value = AppError.Generic("Failed to save Lichess token: ${e.message}")
        }
    }

    /**
     * Save language preference
     */
    fun saveLanguage(language: AppLanguage) {
        try {
            configStore.saveLanguage(language.code)
            _language.value = language
            _error.value = null
        } catch (e: Exception) {
            _error.value = AppError.Generic("Failed to save language: ${e.message}")
        }
    }

    /**
     * Set OtaAction and start collecting its state
     */
    fun setOtaAction(action: OtaAction) {
        this.otaAction = action
        scope.launch {
            action.otaState.collect { state ->
                _otaState.value = state
            }
        }
    }

    /**
     * Upload firmware via OTA
     */
    fun uploadFirmware(data: ByteArray) {
        val action = otaAction
        if (action == null) {
            _error.value = AppError.OtaError.DeviceNotConnected("OTA not available")
            return
        }

        try {
            action.uploadFirmware(data)
            _error.value = null
        } catch (e: Exception) {
            _error.value = AppError.OtaError.UploadFailed(e.message ?: "Upload failed")
        }
    }

    /**
     * Reset OTA status
     */
    fun resetOtaStatus() {
        otaAction?.resetStatus()
    }

    /**
     * Clear error state
     */
    fun clearError() {
        _error.value = null
    }
}
