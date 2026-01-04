package me.aligator.e_chess.service.bluetooth

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.util.Log
import kotlinx.coroutines.cancel
import java.util.UUID

private val LOG_TAG = "ChessBoardDeviceAction"

private val GAME_KEY_CHARACTERISTIC_UUID: UUID = UUID.fromString("0de794de-c3a3-48b8-bd81-893d30342c87")
private val GAME_STATE_CHARACTERISTIC_UUID: UUID = UUID.fromString("ccdffbc5-44ce-41a7-9a15-d70b82f81b1a")

class ChessBoardDeviceAction(
    val ble: Ble,
    private val onGameStateChanged: (String) -> Unit = {}
) : BleAction {
    private var gameKeyCharacteristic: BluetoothGattCharacteristic? = null
    private var gameStateCharacteristic: BluetoothGattCharacteristic? = null
    private var gatt: BluetoothGatt? = null

    init {
        ble.register(this)
    }

    override fun onConnect(gatt: BluetoothGatt, device: SimpleDevice) {
        this.gatt = gatt
    }

    override fun onDisconnect() {
        gameKeyCharacteristic = null
        gameStateCharacteristic = null
        gatt = null
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        if (characteristic.uuid == GAME_STATE_CHARACTERISTIC_UUID) {
            val state = value.decodeToString().trim()
            Log.d(LOG_TAG, "Game state changed: $state")
            onGameStateChanged(state)
        }
    }

    override fun onServiceDiscovered(
        gatt: BluetoothGatt,
        service: BluetoothGattService
    ) {
        gameKeyCharacteristic = service.getCharacteristic(GAME_KEY_CHARACTERISTIC_UUID)
        if (gameKeyCharacteristic == null) {
            Log.e(LOG_TAG, "Could not find the characteristic $GAME_KEY_CHARACTERISTIC_UUID")
            return
        }

        gameStateCharacteristic = service.getCharacteristic(GAME_STATE_CHARACTERISTIC_UUID)
        if (gameStateCharacteristic == null) {
            Log.e(LOG_TAG, "Could not find the characteristic $GAME_STATE_CHARACTERISTIC_UUID")
        } else {
            // Enable notifications for game state changes
            ble.enableNotifications(gatt, gameStateCharacteristic!!)
        }
    }

    fun loadGame(key: String): Boolean {
        if (gatt == null || gameKeyCharacteristic == null) {
            Log.e(LOG_TAG, "Cannot load game since no gatt or gameKey characteristic exists")
            return false
        }
        ble.sendCharacteristic(gatt!!, gameKeyCharacteristic!!, (key + "\n").toByteArray())

        return true
    }

    fun onDestroy() {}
}
