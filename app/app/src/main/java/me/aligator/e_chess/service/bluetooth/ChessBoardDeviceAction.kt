package me.aligator.e_chess.service.bluetooth

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import org.json.JSONObject
import java.util.UUID

private val LOG_TAG = "ChessBoardDeviceAction"

private val ACTION_CHARACTERISTIC_UUID: UUID = UUID.fromString("0de794de-c3a3-48b8-bd81-893d30342c87")
private val EVENT_CHARACTERISTIC_UUID: UUID = UUID.fromString("a1a289ce-d553-4d81-b52d-44e6484507b3")

/**
 * Handles direct communication with the chess board via BLE characteristics.
 *
 * - ACTION characteristic: sends GameCommandEvent commands to the board (write)
 * - EVENT characteristic: receives SerializableGameStateEvent events from the board (notify)
 *
 * All messages are wrapped in a Frame<T> JSON structure: {"v": 1, ...message}
 */
class ChessBoardDeviceAction(
    val ble: Ble,
) : BleAction {
    private var actionCharacteristic: BluetoothGattCharacteristic? = null
    private var eventCharacteristic: BluetoothGattCharacteristic? = null
    private var gatt: BluetoothGatt? = null
    private val eventBuffer = StringBuilder()

    private val _gameLoadState = MutableStateFlow<String?>(null)
    val gameLoadState: StateFlow<String?> = _gameLoadState.asStateFlow()

    private val _ongoingGames = MutableStateFlow<String?>(null)
    val ongoingGames: StateFlow<String?> = _ongoingGames.asStateFlow()

    private val _isLoadingGames = MutableStateFlow(false)
    val isLoadingGames: StateFlow<Boolean> = _isLoadingGames.asStateFlow()

    init {
        ble.register(this)
    }

    override fun onConnect(gatt: BluetoothGatt, device: SimpleDevice) {
        this.gatt = gatt
    }

    override fun onDisconnect() {
        actionCharacteristic = null
        eventCharacteristic = null
        gatt = null
        eventBuffer.clear()
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        if (characteristic.uuid == EVENT_CHARACTERISTIC_UUID) {
            val chunk = value.decodeToString()
            eventBuffer.append(chunk)

            var delimiterIndex = eventBuffer.indexOfAny(charArrayOf('\n', '\r'))
            while (delimiterIndex != -1) {
                val frame = eventBuffer.substring(0, delimiterIndex).trim()
                eventBuffer.delete(0, delimiterIndex + 1)

                if (frame.isNotEmpty()) {
                    Log.d(LOG_TAG, "Received frame: $frame")
                    processEventFrame(frame)
                }

                delimiterIndex = eventBuffer.indexOfAny(charArrayOf('\n', '\r'))
            }
        }
    }

    private fun processEventFrame(frame: String) {
        try {
            val json = JSONObject(frame)
            val version = json.optInt("v", 0)

            if (version != 1) {
                Log.w(LOG_TAG, "Unknown protocol version: $version")
                return
            }

            if (!json.has("type")) {
                Log.w(LOG_TAG, "Frame missing type field, ignoring: $frame")
                return
            }

            val eventType = json.getString("type")

            when (eventType) {
                "ongoing_games_loaded" -> {
                    val gamesArray = json.getJSONArray("games")
                    Log.d(LOG_TAG, "Ongoing games loaded: ${gamesArray.length()} games")
                    _ongoingGames.value = gamesArray.toString()
                    _isLoadingGames.value = false
                }

                "game_loaded" -> {
                    val gameKey = json.getString("game_key")
                    Log.d(LOG_TAG, "Game loaded: $gameKey")
                    _gameLoadState.value = gameKey
                }

                else -> {
                    Log.w(LOG_TAG, "Unknown event type: $eventType")
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to parse event frame: $frame", e)
        }
    }

    override fun onServiceDiscovered(
        gatt: BluetoothGatt,
        service: BluetoothGattService
    ) {
        actionCharacteristic = service.getCharacteristic(ACTION_CHARACTERISTIC_UUID)
        if (actionCharacteristic == null) {
            Log.e(LOG_TAG, "Could not find the ACTION characteristic $ACTION_CHARACTERISTIC_UUID")
            return
        }

        eventCharacteristic = service.getCharacteristic(EVENT_CHARACTERISTIC_UUID)
        if (eventCharacteristic == null) {
            Log.e(LOG_TAG, "Could not find the EVENT characteristic $EVENT_CHARACTERISTIC_UUID")
        } else {
            ble.enableNotifications(gatt, eventCharacteristic!!)
        }
    }

    private fun sendGameCommand(json: JSONObject): Boolean {
        if (gatt == null) {
            Log.w(LOG_TAG, "Cannot send command: not connected")
            return false
        }
        if (actionCharacteristic == null) {
            Log.w(LOG_TAG, "Cannot send command: service discovery not completed yet")
            return false
        }

        try {
            val frame = JSONObject()
            frame.put("v", 1)
            json.keys().forEach { key ->
                frame.put(key, json.get(key))
            }

            val message = frame.toString() + "\n"
            Log.d(LOG_TAG, "Sending command: $message")
            ble.sendCharacteristic(gatt!!, actionCharacteristic!!, message.toByteArray())
            return true
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to send command", e)
            return false
        }
    }

    fun loadGame(key: String): Boolean {
        val command = JSONObject()
        command.put("type", "load_new_game")
        command.put("game_key", key)
        return sendGameCommand(command)
    }

    fun loadOpenGames(): Boolean {
        _isLoadingGames.value = true
        val command = JSONObject()
        command.put("type", "load_open_games")
        return sendGameCommand(command)
    }

    fun onDestroy() {
        eventBuffer.clear()
    }
}
