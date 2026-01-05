package me.aligator.e_chess.ui

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.lang.ref.WeakReference
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import me.aligator.e_chess.service.ConfigurationStore
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.bluetooth.BleState
import org.json.JSONArray
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
    private val configStore = ConfigurationStore(application)

    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    private val _isLoadingGames = MutableStateFlow(false)

    private val _isLoadingGame = MutableStateFlow(false)
    private val _selectedGameKey = MutableStateFlow(configStore.getLastLoadedGame() ?: "")
    private val _bleState = MutableStateFlow(BleState())
    private var bluetoothServiceRef: WeakReference<BluetoothService>? = null

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
            isConnected = bleState.connectedDevice.deviceState == DeviceState.CONNECTED && bleState.connectedDevice.characteristicsReady
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5000),
        initialValue = BleUiState()
    )

    fun setBluetoothService(service: BluetoothService?) {
        bluetoothServiceRef = service?.let { WeakReference(it) }

        if (service != null) {
            viewModelScope.launch {
                service.ble.bleState.collect { state ->
                    _bleState.value = state
                }
            }
            viewModelScope.launch {
                service.chessBoardAction.ongoingGames.collect { gamesJson ->
                    if (gamesJson != null) {
                        try {
                            val games = parseOngoingGames(gamesJson)
                            _availableGames.value = games
                        } catch (e: Exception) {
                            Log.e(LOG_TAG, "Failed to parse ongoing games", e)
                            _availableGames.value = emptyList()
                        }
                    }
                }
            }
            viewModelScope.launch {
                service.chessBoardAction.isLoadingGames.collect { isLoading ->
                    _isLoadingGames.value = isLoading
                }
            }
            viewModelScope.launch {
                service.chessBoardAction.isLoadingGame.collect { isLoading ->
                    _isLoadingGame.value = isLoading
                }
            }
        } else {
            _bleState.value = BleState()
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
        configStore.saveLastLoadedGame(gameKey)
        val success = bluetoothServiceRef?.get()?.chessBoardAction?.loadGame(gameKey)
        Log.d(LOG_TAG, "loadGame sent to board, success: $success")
    }

    fun fetchGames() {
        bluetoothServiceRef?.get()?.chessBoardAction?.loadOpenGames()
    }

    private fun parseOngoingGames(json: String): List<GameOption> {
        val array = JSONArray(json)
        val games = mutableListOf<GameOption>()

        for (i in 0 until array.length()) {
            val game = array.getJSONObject(i)
            val gameId = game.getString("game_id")
            val opponent = game.optJSONObject("opponent")
            val opponentName = opponent?.optString("username") ?: "Unknown"

            games.add(
                GameOption(
                    id = gameId,
                    displayName = "vs $opponentName ($gameId)"
                )
            )
        }

        return games
    }

    fun setSelectedGameKey(key: String) {
        _selectedGameKey.value = key
    }
}
