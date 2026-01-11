package me.aligator.e_chess.repository

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import me.aligator.e_chess.model.AppError
import me.aligator.e_chess.service.bluetooth.Ble
import me.aligator.e_chess.service.bluetooth.BleState
import me.aligator.e_chess.service.bluetooth.SimpleDevice

/**
 * Repository for BLE device management.
 * Single Source of Truth for:
 * - BLE state (scanning, connection, bonding)
 * - Available devices
 * - PIN dialog state
 */
@Suppress("unused") // Injected via Koin DI
class BleRepository {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    // BLE state from Ble service
    private val _bleState = MutableStateFlow(BleState())
    val bleState: StateFlow<BleState> = _bleState.asStateFlow()

    // PIN dialog state
    private val _showPinDialog = MutableStateFlow(false)
    val showPinDialog: StateFlow<Boolean> = _showPinDialog.asStateFlow()

    // Callback for PIN submission
    private var pinSubmitCallback: (suspend (String) -> Unit)? = null

    // Error
    private val _error = MutableStateFlow<AppError?>(null)
    val error: StateFlow<AppError?> = _error.asStateFlow()

    // Ble service reference
    private var ble: Ble? = null

    /**
     * Set Ble service and start collecting its state
     */
    fun setBle(bleService: Ble) {
        this.ble = bleService

        // Collect BLE state from service
        scope.launch {
            bleService.bleState.collect { state ->
                _bleState.value = state
            }
        }

        // Setup PIN request callback
        bleService.onPinRequested = { submitCallback ->
            _showPinDialog.value = true
            pinSubmitCallback = submitCallback
        }
    }

    /**
     * Start scanning for BLE devices
     */
    fun startScan() {
        val bleService = ble
        if (bleService == null) {
            _error.value = AppError.BleError.Unknown("BLE service not initialized")
            return
        }

        try {
            _error.value = null
            bleService.startScan()
        } catch (e: Exception) {
            _error.value = AppError.BleError.ScanFailed(e.message ?: "Failed to start scan")
        }
    }

    /**
     * Stop scanning for BLE devices
     */
    fun stopScan() {
        val bleService = ble
        if (bleService == null) {
            _error.value = AppError.BleError.Unknown("BLE service not initialized")
            return
        }

        try {
            _error.value = null
            bleService.stopScan()
        } catch (e: Exception) {
            _error.value = AppError.BleError.Unknown(e.message ?: "Failed to stop scan")
        }
    }

    /**
     * Connect to a BLE device
     */
    fun connect(device: SimpleDevice) {
        val bleService = ble
        if (bleService == null) {
            _error.value = AppError.BleError.Unknown("BLE service not initialized")
            return
        }

        try {
            _error.value = null
            bleService.connect(device)
        } catch (e: Exception) {
            _error.value = AppError.BleError.ConnectionFailed(e.message ?: "Failed to connect")
        }
    }

    /**
     * Disconnect from current BLE device
     */
    fun disconnect() {
        val bleService = ble
        if (bleService == null) {
            _error.value = AppError.BleError.Unknown("BLE service not initialized")
            return
        }

        try {
            _error.value = null
            bleService.disconnect()
        } catch (e: Exception) {
            _error.value = AppError.BleError.DisconnectFailed(e.message ?: "Failed to disconnect")
        }
    }

    /**
     * Check Bluetooth availability and permissions
     */
    fun checkBluetooth() {
        val bleService = ble
        if (bleService == null) {
            _error.value = AppError.BleError.Unknown("BLE service not initialized")
            return
        }

        try {
            _error.value = null
            bleService.checkBluetooth()
        } catch (e: Exception) {
            _error.value = AppError.BleError.Unknown(e.message ?: "Failed to check bluetooth")
        }
    }

    /**
     * Submit PIN for bonding
     */
    fun submitPin(pin: String) {
        scope.launch {
            try {
                _error.value = null
                pinSubmitCallback?.invoke(pin)
                _showPinDialog.value = false
                pinSubmitCallback = null
            } catch (e: Exception) {
                _error.value = AppError.BleError.BondingFailed(e.message ?: "Failed to submit PIN")
            }
        }
    }

    /**
     * Cancel PIN dialog
     */
    fun cancelPinDialog() {
        _showPinDialog.value = false
        pinSubmitCallback = null
        _error.value = AppError.BleError.BondingFailed("PIN entry cancelled")
    }

    /**
     * Clear error state
     */
    fun clearError() {
        _error.value = null
    }

    /**
     * Reset state
     */
    fun reset() {
        _bleState.value = BleState()
        _showPinDialog.value = false
        _error.value = null
        pinSubmitCallback = null
    }
}
