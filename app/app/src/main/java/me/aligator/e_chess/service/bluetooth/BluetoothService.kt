package me.aligator.e_chess.service.bluetooth

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import java.util.UUID

private const val LOG_TAG = "BluetoothService"

/**
 * UUID of the chess board BLE service.
 * It must match the service id used by the firmware.
 */
private val SERVICE_UUID: UUID = UUID.fromString("b4d75b6c-7284-4268-8621-6e3cef3c6ac4")

class BluetoothService : Service() {
    inner class LocalBinder : Binder() {
        val service: BluetoothService
            get() = this@BluetoothService
    }

    private val binder = LocalBinder()
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    lateinit var ble: Ble

    /**
     * Bridges http requests from the board to an upstream api.
     */
    private lateinit var httpBridge: HttpBleBridge

    /**
     * Connects to the board to set / query the board state.
     */
    private lateinit var chessBoard: ChessBoardDevice


    override fun onCreate() {
        super.onCreate()
        ble = Ble(
            parentScope = serviceScope,
            context = applicationContext,
            serviceUuid = SERVICE_UUID
        )
        ble.checkBluetooth()
        httpBridge = HttpBleBridge(ble)
        chessBoard = ChessBoardDevice(ble)
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onDestroy() {
        ble.onDestroy()
        httpBridge.onDestroy()
        chessBoard.onDestroy()
        super.onDestroy()
    }

    /**
     * Connect to a specific device.
     * Stops the ble scan.
     */
    fun connect(device: SimpleDevice) {
        ble.stopScan()
        httpBridge.connect(device)
        chessBoard.connect(device)
    }
}
