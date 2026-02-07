package me.aligator.e_chess.feature.ble

import kotlinx.coroutines.flow.StateFlow
import me.aligator.e_chess.platform.ble.BleState
import me.aligator.e_chess.platform.ble.BoardBleService
import me.aligator.e_chess.platform.ble.ConnectionStep
import me.aligator.e_chess.platform.ble.ConnectedDevice
import me.aligator.e_chess.platform.ble.DeviceState

interface BleClient {
    val bleState: StateFlow<BleState>
    val ongoingGamesJson: StateFlow<String?>
    val isLoadingGames: StateFlow<Boolean>
    val isLoadingGame: StateFlow<Boolean>
    val boardFen: StateFlow<String?>

    fun startScan()
    fun stopScan()
    fun connect(address: String)
    fun disconnect()
    fun loadGame(gameKey: String): Boolean
    fun loadOpenGames(): Boolean
}

class RealBleClient(
    private val service: BoardBleService
) : BleClient {
    override val bleState: StateFlow<BleState> = service.ble.bleState
    override val ongoingGamesJson: StateFlow<String?> = service.boardControlAction.ongoingGames
    override val isLoadingGames: StateFlow<Boolean> = service.boardControlAction.isLoadingGames
    override val isLoadingGame: StateFlow<Boolean> = service.boardControlAction.isLoadingGame
    override val boardFen: StateFlow<String?> = service.boardControlAction.boardFen

    override fun startScan() = service.ble.startScan()
    override fun stopScan() = service.ble.stopScan()

    override fun connect(address: String) {
        val device = service.ble.bleState.value.devices.firstOrNull { it.address == address } ?: return
        service.ble.connect(device)
    }

    override fun disconnect() = service.ble.disconnect()
    override fun loadGame(gameKey: String): Boolean = service.boardControlAction.loadGame(gameKey)
    override fun loadOpenGames(): Boolean = service.boardControlAction.loadOpenGames()
}

class MockBleClient : BleClient {
    private val _bleState = kotlinx.coroutines.flow.MutableStateFlow(BleState(step = ConnectionStep.IDLE))
    override val bleState: StateFlow<BleState> = _bleState

    private val _ongoingGamesJson = kotlinx.coroutines.flow.MutableStateFlow<String?>(null)
    override val ongoingGamesJson: StateFlow<String?> = _ongoingGamesJson

    private val _isLoadingGames = kotlinx.coroutines.flow.MutableStateFlow(false)
    override val isLoadingGames: StateFlow<Boolean> = _isLoadingGames

    private val _isLoadingGame = kotlinx.coroutines.flow.MutableStateFlow(false)
    override val isLoadingGame: StateFlow<Boolean> = _isLoadingGame

    private val _boardFen = kotlinx.coroutines.flow.MutableStateFlow<String?>(null)
    override val boardFen: StateFlow<String?> = _boardFen

    override fun startScan() {
        _bleState.value = _bleState.value.copy(step = ConnectionStep.SCANNING)
    }

    override fun stopScan() {
        _bleState.value = _bleState.value.copy(step = ConnectionStep.IDLE)
    }

    override fun connect(address: String) {
        _bleState.value = _bleState.value.copy(
            step = ConnectionStep.IDLE,
            connectedDevice = ConnectedDevice(
                deviceState = DeviceState.CONNECTED,
                address = address,
                characteristicsReady = true
            )
        )
    }

    override fun disconnect() {
        _bleState.value = _bleState.value.copy(
            connectedDevice = ConnectedDevice(
                deviceState = DeviceState.DISCONNECTED,
                address = null,
                characteristicsReady = false
            )
        )
        _boardFen.value = null
    }

    override fun loadGame(gameKey: String): Boolean {
        _isLoadingGame.value = true
        _boardFen.value = when (gameKey) {
            "mock-game-1" -> "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            "mock-game-2" -> "r1bqkbnr/pppp1ppp/2n5/4p3/3P4/5N2/PPP1PPPP/RNBQKB1R w KQkq - 0 3"
            else -> "r1bqkbnr/pppppppp/2n5/8/3P4/5N2/PPP1PPPP/RNBQKB1R b KQkq - 1 2"
        }
        _isLoadingGame.value = false
        return true
    }

    override fun loadOpenGames(): Boolean {
        val json = org.json.JSONArray().apply {
            put(org.json.JSONObject().apply {
                put("game_id", "mock-game-1")
                put("opponent", org.json.JSONObject().put("username", "MockBot"))
            })
            put(org.json.JSONObject().apply {
                put("game_id", "mock-game-2")
                put("opponent", org.json.JSONObject().put("username", "MockBot"))
            })
        }
        _ongoingGamesJson.value = json.toString()
        _isLoadingGames.value = false
        return true
    }
}
