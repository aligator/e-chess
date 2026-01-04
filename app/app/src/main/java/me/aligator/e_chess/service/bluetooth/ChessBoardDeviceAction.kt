package me.aligator.e_chess.service.bluetooth

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.util.Log
import java.util.UUID

private val LOG_TAG = "ChessBoardDeviceAction"

private val GAME_KEY_CHARACTERISTIC_UUID: UUID = UUID.fromString("d4f1e338-3396-4e72-a7d7-7c037fbcc0a1")

class ChessBoardDeviceAction(val ble: Ble) : BleAction {
    private var gameKeyCharacteristic: BluetoothGattCharacteristic? = null
    private var gatt: BluetoothGatt? = null

    init {
        ble.register(this)
    }

    override fun onConnect(gatt: BluetoothGatt, device: SimpleDevice) {
        this.gatt = gatt
    }

    override fun onDisconnect() {
        gameKeyCharacteristic = null
        gatt = null
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        // TODO("Not yet implemented")
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
    }

    fun loadGame(key: String): Boolean {
        if (gatt == null || gameKeyCharacteristic == null) {
            Log.e(LOG_TAG, "Cannot load game since no gatt or gameKey characteristic exists")
            return false
        }
        ble.sendCharacteristic(gatt!!, gameKeyCharacteristic!!, (key + "\n").toByteArray())

        return true
    }
}
