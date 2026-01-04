package me.aligator.e_chess.ui

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.LichessApi
import me.aligator.e_chess.service.bluetooth.BleState
import me.aligator.e_chess.service.bluetooth.BluetoothService
import me.aligator.e_chess.service.bluetooth.DeviceState
import me.aligator.e_chess.service.bluetooth.SimpleDevice

private const val LOG_TAG = "BleViewModel"

data class BleUiState(
    val bleState: BleState = BleState(),
    val availableGames: List<GameOption> = emptyList(),
    val isLoadingGames: Boolean = false,
    val isLoadingGame: Boolean = false,
    val selectedGameKey: String = "",
    val isConnected: Boolean = false,
)

class BleViewModel(application: Application) : AndroidViewModel(application) {
    private val lichessApi = LichessApi(application)
    private val configStore = ConfigurationStore(application)

    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    private val _isLoadingGames = MutableStateFlow(false)
    private val _isLoadingGame = MutableStateFlow(false)
    private val _selectedGameKey = MutableStateFlow(configStore.getLastLoadedGame() ?: "")
    private val _bleState = MutableStateFlow(BleState())

    private var bluetoothService: BluetoothService? = null

    val uiState: StateFlow<BleUiState> = combine(
        _bleState,
        _availableGames,
        _isLoadingGames,
        _isLoadingGame,
        _selectedGameKey
    ) { bleState, games, loadingGames, loadingGame, gameKey ->
        BleUiState(
            bleState = bleState,
            availableGames = games,
            isLoadingGames = loadingGames,
            isLoadingGame = loadingGame,
            selectedGameKey = gameKey,
            isConnected = bleState.connectedDevice.deviceState == DeviceState.CONNECTED
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5000),
        initialValue = BleUiState()
    )

    fun setBluetoothService(service: BluetoothService?) {
        bluetoothService = service

        if (service != null) {
            viewModelScope.launch {
                service.ble.bleState.collect { state ->
                    _bleState.value = state
                }
            }
            viewModelScope.launch {
                service.gameLoadState.collect { state ->
                    Log.d(LOG_TAG, "Received game load state: $state, current isLoadingGame: ${_isLoadingGame.value}")
                    when (state) {
                        "loaded" -> {
                            Log.d(LOG_TAG, "Setting isLoadingGame to false (loaded)")
                            _isLoadingGame.value = false
                        }
                        "error" -> {
                            Log.d(LOG_TAG, "Setting isLoadingGame to false (error)")
                            _isLoadingGame.value = false
                        }
                        null -> {} // Initial state, do nothing
                    }
                }
            }
        } else {
            _bleState.value = BleState()
        }
    }

    fun startScan() {
        bluetoothService?.ble?.startScan()
    }

    fun stopScan() {
        bluetoothService?.ble?.stopScan()
    }

    fun connect(device: SimpleDevice) {
        bluetoothService?.ble?.connect(device)
    }

    fun disconnect() {
        Log.d(LOG_TAG, "disconnect called")
        bluetoothService?.ble?.disconnect()
        // Reset loading state
        _isLoadingGame.value = false
    }

    fun loadGame(gameKey: String) {
        Log.d(LOG_TAG, "loadGame called with key: $gameKey")
        _isLoadingGame.value = true
        Log.d(LOG_TAG, "Set isLoadingGame to true")

        // Save the game key for next time
        configStore.saveLastLoadedGame(gameKey)

        val actualGameKey = if (gameKey == "standard") {
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        } else {
            gameKey
        }
        val success = bluetoothService?.chessBoardAction?.loadGame(actualGameKey)
        Log.d(LOG_TAG, "loadGame sent to board, success: $success")
        // Loading state will be cleared when we receive the game state notification
    }

    fun fetchGames() {
        viewModelScope.launch {
            _isLoadingGames.value = true
            try {
                _availableGames.value = lichessApi.getOngoingGames()
            } finally {
                _isLoadingGames.value = false
            }
        }
    }

    fun setSelectedGameKey(key: String) {
        _selectedGameKey.value = key
    }
}
