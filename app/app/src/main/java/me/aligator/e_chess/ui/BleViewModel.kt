package me.aligator.e_chess.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import me.aligator.e_chess.repository.BleRepository
import me.aligator.e_chess.repository.GamesRepository
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.bluetooth.BleState
import me.aligator.e_chess.service.bluetooth.DeviceState
import me.aligator.e_chess.service.bluetooth.SimpleDevice

/**
 * UI State for BLE screen combining BLE and Games state
 */
data class BleUiState(
    val bleState: BleState = BleState(),
    val availableGames: List<GameOption> = emptyList(),
    val isLoadingGames: Boolean = false,
    val isLoadingGame: Boolean = false,
    val selectedGameKey: String? = null,
    val isConnected: Boolean = false,
    val showPinDialog: Boolean = false,
)

/**
 * ViewModel for BLE & Chess screen.
 * Combines BleRepository + GamesRepository state.
 * Uses nested combine() due to Kotlin Flow 5-parameter limit.
 */
class BleViewModel(
    private val bleRepository: BleRepository,
    private val gamesRepository: GamesRepository
) : ViewModel() {

    // Combine 8 StateFlows via nested combine (Flow limit: 5)
    // Group 1: BLE state (4 flows)
    private val bleGroup = combine(
        bleRepository.bleState,
        bleRepository.showPinDialog,
        gamesRepository.availableGames,
        gamesRepository.isLoadingGames
    ) { bleState, showPinDialog, games, loadingGames ->
        BleGroup(bleState, showPinDialog, games, loadingGames)
    }

    // Group 2: Games state (3 flows)
    private val gamesGroup = combine(
        gamesRepository.isLoadingGame,
        gamesRepository.selectedGameKey,
        gamesRepository.error
    ) { loadingGame, gameKey, _ ->
        GamesGroup(loadingGame, gameKey)
    }

    // Combined UI state
    val uiState: StateFlow<BleUiState> = combine(
        bleGroup,
        gamesGroup
    ) { ble, games ->
        BleUiState(
            bleState = ble.bleState,
            availableGames = ble.availableGames,
            isLoadingGames = ble.isLoadingGames,
            isLoadingGame = games.isLoadingGame,
            selectedGameKey = games.selectedGameKey,
            isConnected = ble.bleState.connectedDevice.deviceState == DeviceState.CONNECTED
                && ble.bleState.connectedDevice.characteristicsReady,
            showPinDialog = ble.showPinDialog
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5000),
        initialValue = BleUiState()
    )

    // BLE actions
    fun startScan() = bleRepository.startScan()
    fun stopScan() = bleRepository.stopScan()
    fun connect(device: SimpleDevice) = bleRepository.connect(device)
    fun disconnect() = bleRepository.disconnect()
    fun checkBluetooth() = bleRepository.checkBluetooth()
    fun submitPin(pin: String) = bleRepository.submitPin(pin)
    fun dismissPinDialog() = bleRepository.cancelPinDialog()

    // Games actions
    fun loadAvailableGames() = gamesRepository.loadAvailableGames()
    fun loadGame(gameKey: String) = gamesRepository.selectGame(gameKey)
    fun fetchGames() = gamesRepository.loadOpenGamesOnDevice()
    fun setSelectedGameKey(key: String) {
        // This is handled by selectGame, but kept for compatibility
        // Could be removed if UI doesn't need this
    }

    // Error handling
    fun clearBleError() = bleRepository.clearError()
    fun clearGamesError() = gamesRepository.clearError()
}

// Helper data classes for nested combine
private data class BleGroup(
    val bleState: BleState,
    val showPinDialog: Boolean,
    val availableGames: List<GameOption>,
    val isLoadingGames: Boolean
)

private data class GamesGroup(
    val isLoadingGame: Boolean,
    val selectedGameKey: String?
)
