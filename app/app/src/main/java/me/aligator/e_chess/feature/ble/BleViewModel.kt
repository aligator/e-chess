package me.aligator.e_chess.feature.ble

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.lang.ref.WeakReference
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import kotlinx.coroutines.coroutineScope
import me.aligator.e_chess.data.SettingsStore
import me.aligator.e_chess.data.model.GameOption
import me.aligator.e_chess.platform.ble.BleState
import me.aligator.e_chess.platform.ble.BoardBleService
import me.aligator.e_chess.platform.ble.DeviceState
import me.aligator.e_chess.platform.ble.SimpleDevice
import me.aligator.e_chess.ui.UiEvent
import me.aligator.e_chess.ui.UiMessage
import me.aligator.e_chess.R

private const val LOG_TAG = "BleViewModel"

data class BleUiState(
    val bleState: BleState = BleState(),
    val availableGames: List<GameOption> = emptyList(),
    val isLoadingGames: Boolean = false,
    val isLoadingGame: Boolean = false,
    val selectedGameKey: String = "",
    val isConnected: Boolean = false,
    val lastConnectedAddress: String? = null,
    val lastLoadedGame: String? = null,
)

private data class BleBaseState(
    val bleState: BleState,
    val availableGames: List<GameOption>,
    val isLoadingGames: Boolean,
    val isLoadingGame: Boolean
)

class BleViewModel(application: Application) : AndroidViewModel(application) {
    private val settingsStore = SettingsStore(application)

    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    private val _isLoadingGames = MutableStateFlow(false)

    private val _isLoadingGame = MutableStateFlow(false)
    private val _selectedGameKey = MutableStateFlow(settingsStore.getLastLoadedGame() ?: "")
    private val _lastLoadedGame = MutableStateFlow(settingsStore.getLastLoadedGame())
    private val _lastConnectedAddress = MutableStateFlow(settingsStore.getLastConnectedDeviceAddress())
    private val _bleState = MutableStateFlow(BleState())
    private var bluetoothServiceRef: WeakReference<BoardBleService>? = null
    private var serviceCollectorsJob: Job? = null
    private var currentService: BoardBleService? = null
    private var lastSavedConnectedAddress: String? = _lastConnectedAddress.value
    private val _events = MutableSharedFlow<UiEvent>(extraBufferCapacity = 1)

    val events = _events.asSharedFlow()

    private val baseState = combine(
        _bleState,
        _availableGames,
        _isLoadingGames,
        _isLoadingGame
    ) { bleState, games, loadingGames, loadingGame ->
        BleBaseState(
            bleState = bleState,
            availableGames = games,
            isLoadingGames = loadingGames,
            isLoadingGame = loadingGame
        )
    }

    val uiState: StateFlow<BleUiState> = combine(
        baseState,
        _selectedGameKey,
        _lastConnectedAddress,
        _lastLoadedGame
    ) { base, gameKey, lastAddress, lastGame ->
        BleUiState(
            bleState = base.bleState,
            availableGames = base.availableGames,
            isLoadingGames = base.isLoadingGames,
            isLoadingGame = base.isLoadingGame,
            selectedGameKey = gameKey,
            isConnected = base.bleState.connectedDevice.deviceState == DeviceState.CONNECTED &&
                base.bleState.connectedDevice.characteristicsReady,
            lastConnectedAddress = lastAddress,
            lastLoadedGame = lastGame
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5000),
        initialValue = BleUiState()
    )

    fun setBluetoothService(service: BoardBleService?) {
        if (currentService === service) return
        currentService = service
        bluetoothServiceRef = service?.let { WeakReference(it) }
        serviceCollectorsJob?.cancel()
        serviceCollectorsJob = null

        if (service != null) {
            serviceCollectorsJob = viewModelScope.launch {
                coroutineScope {
                    launch {
                        service.ble.bleState.collect { state ->
                            _bleState.value = state
                            val connectedAddress =
                                if (state.connectedDevice.deviceState == DeviceState.CONNECTED &&
                                    state.connectedDevice.characteristicsReady
                                ) {
                                    state.connectedDevice.address
                                } else {
                                    null
                                }
                            if (!connectedAddress.isNullOrBlank() && connectedAddress != lastSavedConnectedAddress) {
                                lastSavedConnectedAddress = connectedAddress
                                settingsStore.saveLastConnectedDeviceAddress(connectedAddress)
                                _lastConnectedAddress.value = connectedAddress
                            }
                        }
                    }
                    launch {
                        service.boardControlAction.ongoingGames.collect { gamesJson ->
                            if (gamesJson != null) {
                                try {
                                    val games = parseOngoingGames(gamesJson)
                                    _availableGames.value = games
                                } catch (e: Exception) {
                                    Log.e(LOG_TAG, "Failed to parse ongoing games", e)
                                    _availableGames.value = emptyList()
                                }
                            } else {
                                _availableGames.value = emptyList()
                            }
                        }
                    }
                    launch {
                        service.boardControlAction.isLoadingGames.collect { isLoading ->
                            _isLoadingGames.value = isLoading
                        }
                    }
                    launch {
                        service.boardControlAction.isLoadingGame.collect { isLoading ->
                            _isLoadingGame.value = isLoading
                        }
                    }
                }
            }
        } else {
            _bleState.value = BleState()
            _availableGames.value = emptyList()
            _isLoadingGames.value = false
            _isLoadingGame.value = false
        }
    }

    fun startScan() {
        bluetoothServiceRef?.get()?.ble?.startScan()
    }

    fun stopScan() {
        bluetoothServiceRef?.get()?.ble?.stopScan()
    }

    fun connect(device: SimpleDevice) {
        bluetoothServiceRef?.get()?.ble?.connect(device)
    }

    fun disconnect() {
        Log.d(LOG_TAG, "disconnect called")
        bluetoothServiceRef?.get()?.ble?.disconnect()
    }

    fun loadGame(gameKey: String) {
        Log.d(LOG_TAG, "loadGame called with key: $gameKey")
        Log.d(LOG_TAG, "Set isLoadingGame to true")

        // Save the game key for next time
        settingsStore.saveLastLoadedGame(gameKey)
        _lastLoadedGame.value = gameKey
        val success = bluetoothServiceRef?.get()?.boardControlAction?.loadGame(gameKey) ?: false
        Log.d(LOG_TAG, "loadGame sent to board, success: $success")
        val message = if (success) {
            UiMessage.Res(R.string.load_game_sent)
        } else {
            UiMessage.Res(R.string.load_game_failed)
        }
        _events.tryEmit(UiEvent.Snackbar(message))
    }

    fun fetchGames() {
        val success = bluetoothServiceRef?.get()?.boardControlAction?.loadOpenGames() ?: false
        if (!success) {
            _events.tryEmit(UiEvent.Snackbar(UiMessage.Res(R.string.fetch_games_failed)))
        }
    }

    fun setSelectedGameKey(key: String) {
        _selectedGameKey.value = key
    }
}
