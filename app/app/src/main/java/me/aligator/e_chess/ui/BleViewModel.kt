package me.aligator.e_chess.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import me.aligator.e_chess.service.GameOption
import me.aligator.e_chess.service.LichessApi
import me.aligator.e_chess.service.bluetooth.BleState
import me.aligator.e_chess.service.bluetooth.BluetoothService
import me.aligator.e_chess.service.bluetooth.DeviceState
import me.aligator.e_chess.service.bluetooth.SimpleDevice

data class BleUiState(
    val bleState: BleState = BleState(),
    val availableGames: List<GameOption> = emptyList(),
    val isLoadingGames: Boolean = false,
    val selectedGameKey: String = "",
    val isConnected: Boolean = false,
)

class BleViewModel(application: Application) : AndroidViewModel(application) {
    private val lichessApi = LichessApi(application)

    private val _availableGames = MutableStateFlow<List<GameOption>>(emptyList())
    private val _isLoadingGames = MutableStateFlow(false)
    private val _selectedGameKey = MutableStateFlow("")
    private val _bleState = MutableStateFlow(BleState())

    private var bluetoothService: BluetoothService? = null

    val uiState: StateFlow<BleUiState> = combine(
        _bleState,
        _availableGames,
        _isLoadingGames,
        _selectedGameKey
    ) { bleState, games, loading, gameKey ->
        BleUiState(
            bleState = bleState,
            availableGames = games,
            isLoadingGames = loading,
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

    fun loadGame(gameKey: String) {
        val actualGameKey = if (gameKey == "standard") {
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        } else {
            gameKey
        }
        bluetoothService?.chessBoardAction?.loadGame(actualGameKey)
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
