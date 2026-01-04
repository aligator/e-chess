package me.aligator.e_chess.service.bluetooth

import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.content.Context
import android.util.Log
import me.aligator.e_chess.service.LichessTokenStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.BufferedReader
import java.util.UUID
import java.util.concurrent.TimeUnit

private const val LOG_TAG = "HttpBleBridge"

private val BRIDGE_REQUEST_CHARACTERISTIC_UUID = UUID.fromString("aa8381af-049a-46c2-9c92-1db7bd28883c")
private val BRIDGE_RESPONSE_CHARACTERISTIC_UUID = UUID.fromString("29e463e6-a210-4234-8d1d-4daf345b41de")

@Serializable
enum class RequestMethod {
    @SerialName("get")
    GET,

    @SerialName("post")
    POST,

    @SerialName("stream")
    STREAM
}

@Serializable
sealed class BoardToPhone {
    @Serializable
    @SerialName("request")
    data class Request(
        val id: Int,
        val method: RequestMethod,
        val url: String,
        val body: String? = null
    ) : BoardToPhone()

    @Serializable
    @SerialName("cancel")
    data class Cancel(val id: Int) : BoardToPhone()
}

@Serializable
sealed class PhoneToBoard {
    @Serializable
    @SerialName("response")
    data class Response(val id: Int, val body: String) : PhoneToBoard()

    @Serializable
    @SerialName("stream_data")
    data class StreamData(val id: Int, val chunk: String) : PhoneToBoard()

    @Serializable
    @SerialName("stream_closed")
    data class StreamClosed(val id: Int) : PhoneToBoard()
}

private const val PROTOCOL_VERSION = 1

/**
 * The rust firmware needs a way to connect to an upstream chess api.
 * As the direct HTTP connection on the esp32-s3 seems to be quite instable
 * and the connection to a specific wlan makes the handling quite complicated,
 * a HTTP over BLE bridge is used.
 *
 * This bridge is basically just forwarding the url, and body to the specified url.
 * That way the app itself does not need to be modified when adding features to the firmware.
 *
 * However it 'should' only be used for communication between chess board <-> upstream api.
 * For communication between chess board <-> app, normal ble characteristics are used just as
 * they were designed for.
 *
 * ## Bridge inner workings
 * As ble is not really meant for data streaming, there needs to be some higher level protocol
 * implemented on top of the characteristics ble provides.
 *
 * The basis are two characteristics,
 *  * one for sending requests from the board to the api
 *  * and one for receiving the data
 *
 * To make this robust and avoid any confusion between the requests,
 * each request has its own id (assigned by the board firmware) that is also
 * used for the responses.
 *
 * Also since ble is not really designed for concurrent streaming of data,
 * a response queue is used. That way always only one response is sent to the board
 * at a time. However this queue is not coupled to the request order.
 * So there may come two requests in a short time, being sent to upstream concurrently and then
 * the response will be sent as soon as available (using the queue).
 *
 * As (at least) the lichess api uses a long lived request for game state streaming,
 * a special stream mode is implemented. This mode creates a long lived thread and
 * creates a new response for each "line".
 */
class HttpBleBridge(val ble: Ble, context: Context) : BleAction {
    private val tokenStore = LichessTokenStore(context)
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        classDiscriminator = "type"
    }

    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    private val bridgeScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private var gatt: BluetoothGatt? = null
    private var responseCharacteristic: BluetoothGattCharacteristic? = null

    private val requestBuffer = mutableListOf<Byte>()
    private val activeStreams = mutableMapOf<Int, Job>()

    init {
        ble.register(this)
    }

    override fun onConnect(gatt: BluetoothGatt, device: SimpleDevice) {
        this.gatt = gatt
        Log.d(LOG_TAG, "Connected to device: ${device.name}")
    }

    override fun onDisconnect() {
        Log.d(LOG_TAG, "Disconnected from device")

        activeStreams.values.forEach { it.cancel() }
        activeStreams.clear()

        responseCharacteristic = null
        gatt = null
        requestBuffer.clear()
    }

    override fun onServiceDiscovered(gatt: BluetoothGatt, service: BluetoothGattService) {
        val requestCharacteristic = service.getCharacteristic(BRIDGE_REQUEST_CHARACTERISTIC_UUID)
        responseCharacteristic = service.getCharacteristic(BRIDGE_RESPONSE_CHARACTERISTIC_UUID)

        if (requestCharacteristic == null || responseCharacteristic == null) {
            Log.e(LOG_TAG, "Bridge characteristics not found")
            return
        }

        Log.d(LOG_TAG, "Bridge characteristics found, enabling notifications")
        ble.enableNotifications(gatt, requestCharacteristic)
    }

    override fun onCharacteristicChanged(
        gatt: BluetoothGatt,
        characteristic: BluetoothGattCharacteristic,
        value: ByteArray
    ) {
        if (characteristic.uuid != BRIDGE_REQUEST_CHARACTERISTIC_UUID) {
            return
        }

        requestBuffer.addAll(value.toList())

        while (true) {
            val newlineIndex = requestBuffer.indexOfFirst { it == '\n'.code.toByte() || it == '\r'.code.toByte() }
            if (newlineIndex == -1) {
                break
            }

            val messageBytes = requestBuffer.subList(0, newlineIndex).toByteArray()
            requestBuffer.subList(0, newlineIndex + 1).clear()

            if (messageBytes.isEmpty()) {
                continue
            }

            try {
                val messageStr = messageBytes.decodeToString()
                Log.d(LOG_TAG, "Received request: $messageStr")
                handleRequest(messageStr)
            } catch (e: Exception) {
                Log.e(LOG_TAG, "Failed to decode request", e)
            }
        }
    }

    private fun handleRequest(messageStr: String) {
        try {
            val request = json.decodeFromString<BoardToPhone>(messageStr)

            when (request) {
                is BoardToPhone.Request -> {
                    bridgeScope.launch {
                        handleHttpRequest(request)
                    }
                }

                is BoardToPhone.Cancel -> {
                    activeStreams[request.id]?.cancel()
                    activeStreams.remove(request.id)
                    Log.d(LOG_TAG, "Cancelled request ${request.id}")
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to parse request", e)
        }
    }

    private suspend fun handleHttpRequest(request: BoardToPhone.Request) {
        try {
            when (request.method) {
                RequestMethod.GET -> handleGet(request.id, request.url)
                RequestMethod.POST -> handlePost(request.id, request.url, request.body ?: "")
                RequestMethod.STREAM -> handleStream(request.id, request.url)
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "HTTP request failed for id ${request.id}", e)
        }
    }

    private suspend fun handleGet(id: Int, url: String) = withContext(Dispatchers.IO) {
        try {
            val requestBuilder = Request.Builder()
                .url(url)
                .get()

            if (url.startsWith("https://lichess.org/api")) {
                tokenStore.getToken()?.let { token ->
                    requestBuilder.header("Authorization", "Bearer $token")
                }
            }

            val request = requestBuilder.build()

            httpClient.newCall(request).execute().use { response ->
                val body = response.body?.string() ?: ""
                if (response.isSuccessful) {
                    sendResponse(PhoneToBoard.Response(id, body))
                } else {
                    Log.e(LOG_TAG, "GET failed for id $id: HTTP ${response.code}")
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "GET request failed for id $id", e)
        }
    }

    private suspend fun handlePost(id: Int, url: String, body: String) = withContext(Dispatchers.IO) {
        try {
            val requestBody = body.toRequestBody("application/json".toMediaType())
            val requestBuilder = Request.Builder()
                .url(url)
                .post(requestBody)

            if (url.startsWith("https://lichess.org/api")) {
                tokenStore.getToken()?.let { token ->
                    requestBuilder.header("Authorization", "Bearer $token")
                }
            }

            val request = requestBuilder.build()

            httpClient.newCall(request).execute().use { response ->
                val responseBody = response.body?.string() ?: ""
                if (response.isSuccessful) {
                    sendResponse(PhoneToBoard.Response(id, responseBody))
                } else {
                    Log.e(LOG_TAG, "POST failed for id $id: HTTP ${response.code}")
                }
            }
        } catch (e: Exception) {
            Log.e(LOG_TAG, "POST request failed for id $id", e)
        }
    }

    private suspend fun handleStream(id: Int, url: String) {
        val streamJob = bridgeScope.launch(Dispatchers.IO) {
            try {
                val requestBuilder = Request.Builder()
                    .url(url)
                    .get()

                if (url.startsWith("https://lichess.org/api")) {
                    tokenStore.getToken()?.let { token ->
                        requestBuilder.header("Authorization", "Bearer $token")
                    }
                }

                val request = requestBuilder.build()

                httpClient.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) {
                        Log.e(LOG_TAG, "STREAM failed for id $id: HTTP ${response.code}")
                        return@launch
                    }

                    val reader = response.body?.byteStream()?.bufferedReader()
                    if (reader == null) {
                        Log.e(LOG_TAG, "No response body for stream id $id")
                        return@launch
                    }

                    try {
                        reader.use {
                            processStreamLines(it, id)
                        }
                    } catch (err: java.lang.Exception){
                        Log.e(LOG_TAG, "could not process stream lines $err")
                    } finally {
                        Log.d(LOG_TAG, "CLOSE stream")
                        sendResponse(PhoneToBoard.StreamClosed(id))
                        activeStreams.remove(id)
                    }
                }
            } catch (e: Exception) {
                Log.e(LOG_TAG, "Stream request failed for id $id", e)
                activeStreams.remove(id)
            }
        }

        activeStreams[id] = streamJob
    }

    private suspend fun processStreamLines(reader: BufferedReader, id: Int) {
        while (currentCoroutineContext().isActive) {
            Log.d(LOG_TAG, "get line")
            val line = reader.readLine() ?: break
            Log.d(LOG_TAG, "get line $line")
            val trimmed = line.trim()

            if (trimmed.isNotEmpty()) {
                // Add newline back so firmware can properly parse the chunk
                sendResponse(PhoneToBoard.StreamData(id, trimmed + "\n"))
                delay(10)
            }
        }
    }

    private fun sendResponse(response: PhoneToBoard) {
        val currentGatt = gatt
        val currentResponseChar = responseCharacteristic

        if (currentGatt == null || currentResponseChar == null) {
            Log.e(LOG_TAG, "Cannot send response: not connected")
            return
        }

        try {
            // Serialize the response message
            val messageJson = json.encodeToString(response)

            // Parse it to add the version field at the top level
            val messageMap = json.decodeFromString<Map<String, kotlinx.serialization.json.JsonElement>>(messageJson)
            val frameMap = buildMap {
                put("v", kotlinx.serialization.json.JsonPrimitive(PROTOCOL_VERSION))
                putAll(messageMap)
            }

            val responseStr = json.encodeToString(frameMap) + "\n"
            Log.d(LOG_TAG, "Sending response: $responseStr")

            ble.sendCharacteristic(
                currentGatt,
                currentResponseChar,
                responseStr.toByteArray()
            )
        } catch (e: Exception) {
            Log.e(LOG_TAG, "Failed to send response", e)
        }
    }

    fun onDestroy() {
        bridgeScope.cancel()
        activeStreams.values.forEach { it.cancel() }
        activeStreams.clear()
    }
}
