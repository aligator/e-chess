package me.aligator.e_chess.platform.ble

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

private const val LOG_TAG = "BoardOtaAction"

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

class BoardOtaAction(
    private val ble: BleManager
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

                // Protocol: First message is 4 bytes size as u32 little-endian, then data messages follow
                val sizeBytes = ByteArray(4)
                sizeBytes[0] = (data.size and 0xFF).toByte()
                sizeBytes[1] = ((data.size shr 8) and 0xFF).toByte()
                sizeBytes[2] = ((data.size shr 16) and 0xFF).toByte()
                sizeBytes[3] = ((data.size shr 24) and 0xFF).toByte()

                Log.d(LOG_TAG, "Starting OTA upload, size: ${data.size} bytes")

                val currentGatt = ble.gatt
                if (currentGatt != null) {
                    // First send size only
                    ble.sendCharacteristic(currentGatt, characteristic, sizeBytes)

                    // Then send the data with progress tracking
                    ble.sendCharacteristicWithProgress(currentGatt, characteristic, data) { bytesSent, total ->
                        val progress = (bytesSent.toFloat() / total).coerceAtMost(0.95f)
                        _otaState.value = OtaState(
                            status = OtaStatus.UPLOADING,
                            totalBytes = total,
                            bytesUploaded = bytesSent,
                            progress = progress
                        )
                    }
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

    private fun trackProgress(totalBytes: Long) {
        scope.launch {
            // Track progress based on time estimate
            // Assume ~20KB/s transfer rate (conservative estimate)
            val estimatedDurationMs = (totalBytes / 20).toInt() // totalBytes in bytes, rate in KB/s
            val updateIntervalMs = 500L
            val totalUpdates = (estimatedDurationMs / updateIntervalMs).coerceAtLeast(1)

            var updateCount = 0
            while (updateCount < totalUpdates && _otaState.value.status == OtaStatus.UPLOADING) {
                kotlinx.coroutines.delay(updateIntervalMs)
                updateCount++
                val progress = (updateCount.toFloat() / totalUpdates).coerceAtMost(0.95f)
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
