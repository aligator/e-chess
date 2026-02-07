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
import me.aligator.e_chess.ui.UiEvent
import me.aligator.e_chess.ui.UiMessage
import me.aligator.e_chess.R

private const val LOG_TAG = "BleViewModel"

data class BleUiState(
    val bleState: BleState = BleState(),
    val devices: List<BleDeviceItem> = emptyList(),
    val availableGames: List<GameOption> = emptyList(),
    val isLoadingGames: Boolean = false,
    val isLoadingGame: Boolean = false,
    val selectedGameKey: String = "",
    val isConnected: Boolean = false,
    val boardFen: String? = null,
)

private data class BleBaseState(
    val bleState: BleState,
    val devices: List<BleDeviceItem>,
    val availableGames: List<GameOption>,
    val isLoadingGames: Boolean,
    val isLoadingGame: Boolean,
    val boardFen: String?
)

class BleViewModel(application: Application) : AndroidViewModel(application) {
    private val mockDevices = listOf(
        BleDeviceItem("Mock Board A", "00:11:22:33:44:55"),
        BleDeviceItem("Mock Board B", "AA:BB:CC:DD:EE:FF")
    )

    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    private val _isLoadingGames = MutableStateFlow(false)

    private val _isLoadingGame = MutableStateFlow(false)
    private val _boardFen = MutableStateFlow<String?>(null)
    private val _devices = MutableStateFlow<List<BleDeviceItem>>(emptyList())
    private val _selectedGameKey = MutableStateFlow("")
    private val _bleState = MutableStateFlow(BleState())
    private var bluetoothServiceRef: WeakReference<BoardBleService>? = null
    private var serviceCollectorsJob: Job? = null
    private val _events = MutableSharedFlow<UiEvent>(extraBufferCapacity = 1)
    private var mockEnabled = false
    private var currentClient: BleClient? = null

    val events = _events.asSharedFlow()

    private val baseState = combine(
        _bleState,
        _devices,
        _availableGames,
        _isLoadingGames,
        _isLoadingGame
    ) { bleState, devices, games, loadingGames, loadingGame ->
        BleBaseState(
            bleState = bleState,
            devices = devices,
            availableGames = games,
            isLoadingGames = loadingGames,
            isLoadingGame = loadingGame,
            boardFen = _boardFen.value
        )
    }

    val uiState: StateFlow<BleUiState> = combine(
        baseState,
        _selectedGameKey,
        _boardFen
    ) { base, gameKey, boardFen ->
        BleUiState(
            bleState = base.bleState,
            devices = base.devices,
            availableGames = base.availableGames,
            isLoadingGames = base.isLoadingGames,
            isLoadingGame = base.isLoadingGame,
            selectedGameKey = gameKey,
            isConnected = base.bleState.connectedDevice.deviceState == DeviceState.CONNECTED &&
                    base.bleState.connectedDevice.characteristicsReady,
            boardFen = boardFen
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5000),
        initialValue = BleUiState()
    )

    fun setBluetoothService(service: BoardBleService?) {
        val current = bluetoothServiceRef?.get()
        if (current === service) return
        bluetoothServiceRef = service?.let { WeakReference(it) }
        if (!mockEnabled) {
            setClient(service?.let { RealBleClient(it) })
        }

        if (service == null && !mockEnabled) {
            clearClientState()
        }
    }

    fun setMockMode(enabled: Boolean) {
        mockEnabled = enabled
        if (enabled) {
            setClient(MockBleClient())
            _devices.value = mockDevices
        } else {
            val service = bluetoothServiceRef?.get()
            setClient(service?.let { RealBleClient(it) })
            if (service == null) {
                clearClientState()
            }
        }
    }

    fun startScan() {
        currentClient?.startScan()
    }

    fun stopScan() {
        currentClient?.stopScan()
    }

    fun connect(device: BleDeviceItem) {
        currentClient?.connect(device.address)
    }

    fun disconnect() {
        Log.d(LOG_TAG, "disconnect called")
        currentClient?.disconnect()
    }

    fun loadGame(gameKey: String) {
        Log.d(LOG_TAG, "loadGame called with key: $gameKey")
        Log.d(LOG_TAG, "Set isLoadingGame to true")

        val success = currentClient?.loadGame(gameKey) ?: false
        Log.d(LOG_TAG, "loadGame sent to board, success: $success")
        val message = if (success) {
            UiMessage.Res(R.string.load_game_sent)
        } else {
            UiMessage.Res(R.string.load_game_failed)
        }
        _events.tryEmit(UiEvent.Snackbar(message))
    }

    fun fetchGames() {
        val success = currentClient?.loadOpenGames() ?: false
        if (!success) {
            _events.tryEmit(UiEvent.Snackbar(UiMessage.Res(R.string.fetch_games_failed)))
        }
    }

    fun setSelectedGameKey(key: String) {
        _selectedGameKey.value = key
    }

    private fun setClient(client: BleClient?) {
        serviceCollectorsJob?.cancel()
        serviceCollectorsJob = null
        currentClient = client
        if (client == null) return

        serviceCollectorsJob = viewModelScope.launch {
            coroutineScope {
                launch {
                    client.bleState.collect { state ->
                        _bleState.value = state
                        val mappedDevices = state.devices.map {
                            BleDeviceItem(name = it.name, address = it.address)
                        }
                        _devices.value =
                            if (mappedDevices.isNotEmpty()) mappedDevices else if (mockEnabled) mockDevices else mappedDevices
                    }
                }
                launch {
                    client.ongoingGamesJson.collect { gamesJson ->
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
                    client.isLoadingGames.collect { isLoading ->
                        _isLoadingGames.value = isLoading
                    }
                }
                launch {
                    client.isLoadingGame.collect { isLoading ->
                        _isLoadingGame.value = isLoading
                    }
                }
                launch {
                    client.boardFen.collect { fen ->
                        _boardFen.value = fen
                    }
                }
            }
        }
    }

    private fun clearClientState() {
        _bleState.value = BleState()
        _devices.value = emptyList()
        _availableGames.value = emptyList()
        _isLoadingGames.value = false
        _isLoadingGame.value = false
        _boardFen.value = null
    }
}
