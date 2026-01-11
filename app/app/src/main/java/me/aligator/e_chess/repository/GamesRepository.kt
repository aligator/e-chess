package me.aligator.e_chess.repository

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.model.AppError
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.LichessApi
import me.aligator.e_chess.service.bluetooth.ChessBoardDeviceAction

/**
 * Repository for game management and Lichess API integration.
 * Single Source of Truth for:
 * - Available games list
 * - Selected game
 * - Loading states
 * - ChessBoard device communication
 */
class GamesRepository(
    private val lichessApi: LichessApi
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    // Available games from Lichess
    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    val availableGames: StateFlow<List<GameOption>> = _availableGames.asStateFlow()

    // Selected game key
    private val _selectedGameKey = MutableStateFlow<String?>(null)
    val selectedGameKey: StateFlow<String?> = _selectedGameKey.asStateFlow()

    // Loading states
    private val _isLoadingGames = MutableStateFlow(false)
    val isLoadingGames: StateFlow<Boolean> = _isLoadingGames.asStateFlow()

    private val _isLoadingGame = MutableStateFlow(false)
    val isLoadingGame: StateFlow<Boolean> = _isLoadingGame.asStateFlow()

    // Error
    private val _error = MutableStateFlow<AppError?>(null)
    val error: StateFlow<AppError?> = _error.asStateFlow()

    // ChessBoard device action reference
    private var chessBoardAction: ChessBoardDeviceAction? = null

    /**
     * Set ChessBoardDeviceAction and start collecting its state
     */
    fun setChessBoardAction(action: ChessBoardDeviceAction) {
        this.chessBoardAction = action

        // Collect loading states from device
        scope.launch {
            action.isLoadingGames.collect { isLoading ->
                _isLoadingGames.value = isLoading
            }
        }

        scope.launch {
            action.isLoadingGame.collect { isLoading ->
                _isLoadingGame.value = isLoading
            }
        }

        scope.launch {
            action.ongoingGames.collect { gamesJson ->
                if (gamesJson != null) {
                    // Games received from device - we could parse this if needed
                    // For now, just clear the loading state (handled by action.isLoadingGames)
                }
            }
        }
    }

    /**
     * Load available games from Lichess API
     */
    fun loadAvailableGames() {
        scope.launch {
            try {
                _isLoadingGames.value = true
                _error.value = null

                val games = lichessApi.getOngoingGames()
                _availableGames.value = games

                _isLoadingGames.value = false
            } catch (e: Exception) {
                _isLoadingGames.value = false
                _error.value = AppError.ApiError.NetworkError(e.message ?: "Failed to load games")
            }
        }
    }

    /**
     * Select and load a game on the chess board device
     */
    fun selectGame(gameKey: String) {
        val action = chessBoardAction
        if (action == null) {
            _error.value = AppError.BleError.DeviceNotFound("Chess board not connected")
            return
        }

        try {
            _selectedGameKey.value = gameKey
            _isLoadingGame.value = true
            _error.value = null

            val success = action.loadGame(gameKey)
            if (!success) {
                _isLoadingGame.value = false
                _error.value = AppError.BleError.ConnectionFailed("Failed to load game on device")
            }
        } catch (e: Exception) {
            _isLoadingGame.value = false
            _error.value = AppError.BleError.Unknown(e.message ?: "Failed to load game")
        }
    }

    /**
     * Request the device to load available games
     */
    fun loadOpenGamesOnDevice() {
        val action = chessBoardAction
        if (action == null) {
            _error.value = AppError.BleError.DeviceNotFound("Chess board not connected")
            return
        }

        try {
            _error.value = null
            val success = action.loadOpenGames()
            if (!success) {
                _error.value = AppError.BleError.ConnectionFailed("Failed to request games from device")
            }
        } catch (e: Exception) {
            _error.value = AppError.BleError.Unknown(e.message ?: "Failed to load games on device")
        }
    }

    /**
     * Clear error state
     */
    fun clearError() {
        _error.value = null
    }

    /**
     * Reset state when disconnected
     */
    fun reset() {
        _availableGames.value = emptyList()
        _selectedGameKey.value = null
        _isLoadingGames.value = false
        _isLoadingGame.value = false
        _error.value = null
    }
}
