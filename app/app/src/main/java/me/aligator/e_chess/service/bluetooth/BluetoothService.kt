package me.aligator.e_chess.service.bluetooth

import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import me.aligator.e_chess.repository.BleRepository
import me.aligator.e_chess.repository.GamesRepository
import me.aligator.e_chess.repository.SettingsRepository
import org.koin.android.ext.android.inject
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

    // Koin injected repositories
    private val settingsRepository: SettingsRepository by inject()
    private val gamesRepository: GamesRepository by inject()
    private val bleRepository: BleRepository by inject()

    lateinit var ble: Ble

    /**
     * Bridges http requests from the board to an upstream api.
     */
    private lateinit var httpBridgeAction: HttpBleBridgeAction

    /**
     * Connects to the board to set / query the board state.
     */
    lateinit var chessBoardAction: ChessBoardDeviceAction

    /**
     * Handles OTA firmware updates.
     */
    lateinit var otaAction: OtaAction


    override fun onCreate() {
        super.onCreate()
        ble = Ble(
            parentScope = serviceScope,
            context = applicationContext,
            serviceUuid = SERVICE_UUID
        )
        ble.checkBluetooth()
        httpBridgeAction = HttpBleBridgeAction(ble, applicationContext)
        chessBoardAction = ChessBoardDeviceAction(ble)
        otaAction = OtaAction(ble)

        // Inject actions into repositories
        bleRepository.setBle(ble)
        settingsRepository.setOtaAction(otaAction)
        gamesRepository.setChessBoardAction(chessBoardAction)
    }

    override fun onBind(intent: Intent?): IBinder = binder

    override fun onDestroy() {
        httpBridgeAction.onDestroy()
        chessBoardAction.onDestroy()
        otaAction.onDestroy()
        ble.onDestroy()
        super.onDestroy()
    }
}
