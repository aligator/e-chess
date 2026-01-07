package me.aligator.e_chess.service.bluetooth

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.util.UUID

private const val LOG_TAG = "OtaAction"

private val OTA_ACTION_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("5952abbd-0d7d-4f2d-b0bc-8b3ac5fb8686")
private val OTA_EVENT_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("4d46d598-6141-448c-92bd-fed799efaceb")

@Serializable
@SerialName("ota_started")
data class OtaStartedEvent(val size: Long)

@Serializable
@SerialName("ota_complete")
data object OtaCompleteEvent

@Serializable
@SerialName("ota_error")
data class OtaErrorEvent(val message: String)

enum class OtaStatus {
    IDLE,
    UPLOADING,
    COMPLETED,
    ERROR
}

data class OtaState(
    val status: OtaStatus = OtaStatus.IDLE,
    val errorMessage: String? = null,
    val progress: Float = 0f, // 0.0 to 1.0
    val bytesUploaded: Long = 0,
    val totalBytes: Long = 0
)

class OtaAction(
    private val ble: Ble
) : BleAction {
    private val json = Json { ignoreUnknownKeys = true }
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private val _otaState = MutableStateFlow(OtaState())
    val otaState: StateFlow<OtaState> = _otaState.asStateFlow()

    private var otaActionCharacteristic: BluetoothGattCharacteristic? = null
    private var otaEventCharacteristic: BluetoothGattCharacteristic? = null
    private var eventBuffer = StringBuilder()

    init {
        ble.register(this)
    }

    override fun onConnect(gatt: BluetoothGatt, device: SimpleDevice) {
        Log.d(LOG_TAG, "Connected to device")
    }

    override fun onDisconnect() {
        Log.d(LOG_TAG, "Disconnected from device")
        otaActionCharacteristic = null
        otaEventCharacteristic = null
        eventBuffer.clear()
        _otaState.value = OtaState()
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        if (characteristic.uuid == OTA_EVENT_CHARACTERISTIC_UUID) {
            handleEventChunk(value)
        }
    }

    override fun onServiceDiscovered(gatt: BluetoothGatt, service: BluetoothGattService) {
        otaActionCharacteristic = service.getCharacteristic(OTA_ACTION_CHARACTERISTIC_UUID)
        otaEventCharacteristic = service.getCharacteristic(OTA_EVENT_CHARACTERISTIC_UUID)

        if (otaEventCharacteristic != null) {
            ble.enableNotifications(gatt, otaEventCharacteristic!!)
        }

        Log.d(LOG_TAG, "OTA characteristics discovered")
    }

    private fun handleEventChunk(chunk: ByteArray) {
        val str = chunk.decodeToString()

        for (char in str) {
            if (char == '\n') {
                val line = eventBuffer.toString()
                eventBuffer.clear()

                if (line.isNotEmpty()) {
                    handleEventLine(line)
                }
            } else {
                eventBuffer.append(char)
            }
        }
    }

    private fun handleEventLine(line: String) {
        try {
            val jsonElement = json.parseToJsonElement(line)
            val typeValue = jsonElement.jsonObject["type"]?.jsonPrimitive?.content

            when (typeValue) {
                "ota_started" -> {
                    val event = json.decodeFromString<OtaStartedEvent>(line)
                    Log.d(LOG_TAG, "OTA started, size: ${event.size}")
                    _otaState.value = OtaState(status = OtaStatus.UPLOADING)
                }

                "ota_complete" -> {
                    Log.d(LOG_TAG, "OTA completed")
                    _otaState.value = OtaState(status = OtaStatus.COMPLETED)
                }

                "ota_error" -> {
                    val event = json.decodeFromString<OtaErrorEvent>(line)
                    Log.e(LOG_TAG, "OTA error: ${event.message}")
                    _otaState.value = OtaState(
                        status = OtaStatus.ERROR,
                        errorMessage = event.message
                    )
                }

                else -> {
                    Log.w(LOG_TAG, "Unknown event type: $typeValue")
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to parse OTA event: $line", e)
        }
    }

    fun uploadFirmware(data: ByteArray, fileSize: Long = data.size.toLong()) {
        scope.launch {
            try {
                val gatt = ble.bleState.value.connectedDevice
                if (gatt.deviceState != DeviceState.CONNECTED) {
                    _otaState.value = OtaState(
                        status = OtaStatus.ERROR,
                        errorMessage = "Not connected to device"
                    )
                    return@launch
                }

                val characteristic = otaActionCharacteristic
                if (characteristic == null) {
                    _otaState.value = OtaState(
                        status = OtaStatus.ERROR,
                        errorMessage = "OTA characteristic not available"
                    )
                    return@launch
                }

                val totalBytes = data.size.toLong()
                _otaState.value = OtaState(
                    status = OtaStatus.UPLOADING,
                    totalBytes = totalBytes,
                    bytesUploaded = 0,
                    progress = 0f
                )

                // Protocol: First bytes until space are ASCII size, rest is binary firmware data.
                val sizeStr = data.size.toString()
                val header = "$sizeStr ".toByteArray()
                val payload = header + data

                Log.d(LOG_TAG, "Starting OTA upload, size: ${data.size} bytes")

                // Send the entire payload - the Ble class will handle chunking
                // We simulate progress since we don't have chunk-level callbacks
                val currentGatt = ble.gatt
                if (currentGatt != null) {
                    // Start progress simulation
                    simulateProgress(totalBytes)
                    ble.sendCharacteristic(currentGatt, characteristic, payload)
                } else {
                    _otaState.value = OtaState(
                        status = OtaStatus.ERROR,
                        errorMessage = "GATT connection lost"
                    )
                }
            } catch (e: Exception) {
                Log.e(LOG_TAG, "Failed to upload firmware", e)
                _otaState.value = OtaState(
                    status = OtaStatus.ERROR,
                    errorMessage = e.message ?: "Unknown error"
                )
            }
        }
    }

    private fun simulateProgress(totalBytes: Long) {
        scope.launch {
            // Simulate progress until we get actual feedback from firmware
            var progress = 0f
            while (progress < 0.95f && _otaState.value.status == OtaStatus.UPLOADING) {
                kotlinx.coroutines.delay(100)
                progress += 0.05f
                val bytesUploaded = (totalBytes * progress).toLong()
                _otaState.value = _otaState.value.copy(
                    progress = progress,
                    bytesUploaded = bytesUploaded
                )
            }
        }
    }

    fun resetStatus() {
        _otaState.value = OtaState()
    }

    fun onDestroy() {
        // Nothing to clean up
    }
}
