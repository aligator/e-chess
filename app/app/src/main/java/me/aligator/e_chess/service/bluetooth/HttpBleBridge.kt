package me.aligator.e_chess.service.bluetooth

import java.util.UUID

private val BRIDGE_REQUEST_CHARACTERISTIC_UUID = UUID.fromString("aa8381af-049a-46c2-9c92-1db7bd28883c")
private val BRIDGE_RESPONSE_CHARACTERISTIC_UUID = UUID.fromString("29e463e6-a210-4234-8d1d-4daf345b41de")

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
class HttpBleBridge(val ble: Ble) {
    fun onDestroy() {

    }

    fun connect(device: SimpleDevice) {

    }
}