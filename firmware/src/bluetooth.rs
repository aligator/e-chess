//! Bluetooth LE transport for chess game requester.
//! Implements a protocol over BLE GATT characteristics to send HTTP-like
//! requests from the chess board to a connected client, which performs
//! the actual network requests and streams data back to the board.
use std::{
    str,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{game::GameCommandEvent, Event};
use chess_game::requester::Requester;
use esp32_nimble::{uuid128, BLEAdvertisementData, BLECharacteristic, BLEDevice, NimbleProperties};
use log::*;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;
pub const SERVICE_UUID: &str = "b4d75b6c-7284-4268-8621-6e3cef3c6ac4";
pub const BRIDGE_REQUEST_CHARACTERISTIC_UUID: &str = "aa8381af-049a-46c2-9c92-1db7bd28883c";
pub const BRIDGE_RESPONSE_CHARACTERISTIC_UUID: &str = "29e463e6-a210-4234-8d1d-4daf345b41de";
pub const GAME_KEY_CHARACTERISTIC_UUID: &str = "0de794de-c3a3-48b8-bd81-893d30342c87";

// TODO: can I increase the MTU?
// Keep notifications within the lowest possible BLE ATT MTU (20 bytes -> 23 byte payload).
const MIN_MTU_PAYLOAD: usize = 20;

#[derive(Debug)]
pub enum BluetoothError {
    Transport(String),
    Timeout,
    Protocol(String),
}

impl std::fmt::Display for BluetoothError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BluetoothError::Transport(msg) => write!(f, "transport error: {}", msg),
            BluetoothError::Timeout => write!(f, "timeout waiting for response"),
            BluetoothError::Protocol(msg) => write!(f, "protocol error: {}", msg),
        }
    }
}

impl std::error::Error for BluetoothError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum RequestMethod {
    Get,
    Post,
    Stream,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoardToPhone {
    Request {
        id: u32,
        method: RequestMethod,
        url: String,
        body: Option<String>,
    },
    Cancel {
        id: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhoneToBoard {
    Response { id: u32, body: String },
    StreamData { id: u32, chunk: String },
    StreamClosed { id: u32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Frame<T> {
    pub v: u8,
    #[serde(flatten)]
    pub msg: T,
}

pub fn encode_frame(msg: &BoardToPhone) -> Result<Vec<u8>, BluetoothError> {
    serde_json::to_string(&Frame {
        v: PROTOCOL_VERSION,
        msg: msg.clone(),
    })
    .map(|mut body| {
        body.push('\n');
        body.into_bytes()
    })
    .map_err(|e| BluetoothError::Protocol(e.to_string()))
}

pub fn decode_frame(payload: &[u8]) -> Result<PhoneToBoard, BluetoothError> {
    let without_newline = payload
        .iter()
        .copied()
        .take_while(|b| *b != b'\n' && *b != b'\r')
        .collect::<Vec<u8>>();

    serde_json::from_slice::<Frame<PhoneToBoard>>(&without_newline)
        .map(|frame| frame.msg)
        .map_err(|e| BluetoothError::Protocol(e.to_string()))
}

pub trait Transport: Send + Sync {
    fn send(&self, msg: BoardToPhone) -> Result<(), BluetoothError>;
    fn recv(&self) -> Result<PhoneToBoard, BluetoothError>;
}

/// Channel-based transport: integrate BLE callbacks by writing decoded
/// `PhoneToBoard` messages into `to_board` and reading `BoardToPhone`
/// notifications from `from_board`.
#[derive(Clone)]
pub struct ChannelTransport {
    from_board: Sender<BoardToPhone>,
    to_board: Arc<Mutex<Receiver<PhoneToBoard>>>,
}

impl ChannelTransport {
    pub fn new(from_board: Sender<BoardToPhone>, to_board: Receiver<PhoneToBoard>) -> Self {
        Self {
            from_board,
            to_board: Arc::new(Mutex::new(to_board)),
        }
    }
}

impl Transport for ChannelTransport {
    fn send(&self, msg: BoardToPhone) -> Result<(), BluetoothError> {
        info!("{:?}", msg);
        self.from_board
            .send(msg)
            .map_err(|e| BluetoothError::Transport(e.to_string()))
    }

    fn recv(&self) -> Result<PhoneToBoard, BluetoothError> {
        self.to_board
            .lock()
            .unwrap()
            .recv()
            .map_err(|_| BluetoothError::Transport("ble link closed".into()))
    }
}

struct BluetoothInner {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
    next_request_id: AtomicU32,
    // Registry of active requests: maps request_id -> channel to send responses
    request_channels: Arc<Mutex<std::collections::HashMap<u32, Sender<PhoneToBoard>>>>,
}

#[derive(Clone)]
pub struct Bluetooth {
    inner: Arc<BluetoothInner>,
    is_connected: Arc<Mutex<bool>>,
    game_load_tx: Arc<Mutex<Sender<Event>>>,
}

/// Append incoming bytes to `buffer` and extract complete frames terminated by
/// `\n` or `\r`.
///
/// Behavior:
/// - Incoming `data` is appended to the mutable `buffer`.
/// - The function searches `buffer` for delimiters (`\n` or `\r`). For each
///   delimiter found it drains the slice up to and including the delimiter and
///   returns that drained slice as a `Vec<u8>` (so each returned frame contains
///   the delimiter at the end).
/// - Any trailing bytes in `buffer` after the last delimiter are left in place
///   (these represent a partial frame to be completed by subsequent calls).
///
/// Important notes:
/// - This utility operates on raw bytes (Vec<u8>) and does not attempt UTF-8
///   validation or conversion. Callers must decide how to interpret the bytes
///   (e.g., convert to UTF-8 with lossy replacement if needed).
/// - Frames are returned exactly as drained; the function does not trim
///   whitespace or merge multiple delimiters.
/// - This intentionally mirrors the simple delimiter-based logic used in the
///   BLE `on_write` handler: append, find delimiter position, drain(..=pos),
///   and collect.
fn decode_chunked(data: &[u8], buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    // Append new data into the buffer
    buffer.extend_from_slice(data);

    let mut frames: Vec<Vec<u8>> = Vec::new();

    // Drain complete frames/lines using the simple delimiter-based logic.
    while let Some(pos) = buffer.iter().position(|b| *b == b'\n' || *b == b'\r') {
        let frame: Vec<u8> = buffer.drain(..=pos).collect();
        frames.push(frame);
    }

    frames
}

/// Central dispatcher thread that routes incoming messages to the right request handlers
fn dispatch_messages(
    transport: Arc<dyn Transport>,
    request_channels: Arc<Mutex<std::collections::HashMap<u32, Sender<PhoneToBoard>>>>,
) {
    thread::spawn(move || {
        info!("Dispatcher thread started");
        loop {
            match transport.recv() {
                Ok(msg) => {
                    // Extract the ID from the message
                    let id = match &msg {
                        PhoneToBoard::Response { id, .. } => *id,
                        PhoneToBoard::StreamData { id, .. } => *id,
                        PhoneToBoard::StreamClosed { id } => *id,
                    };

                    info!("Dispatcher: received message for id {}: {:?}", id, msg);

                    // Find the channel for this request ID and send the message
                    let channels = request_channels.lock().unwrap();
                    if let Some(tx) = channels.get(&id) {
                        info!("Dispatcher: routing message to request {}", id);
                        if let Err(e) = tx.send(msg) {
                            warn!(
                                "Dispatcher: failed to send message to request {}: {:?}",
                                id, e
                            );
                        }
                    } else {
                        warn!(
                            "Dispatcher: received message for unknown request id {}: {:?}",
                            id, msg
                        );
                    }
                }
                Err(e) => {
                    info!("Dispatcher: transport closed: {:?}", e);
                    break;
                }
            }
        }
        info!("Dispatcher thread exiting");
    });
}

impl Bluetooth {
    pub fn create_and_spawn(
        device_name: &str,
        request_timeout: Duration,
        game_load_tx: Sender<Event>,
    ) -> Self {
        let (to_phone_tx, to_phone_rx) = std::sync::mpsc::channel();
        let (from_phone_tx, from_phone_rx) = std::sync::mpsc::channel();
        let transport = Arc::new(ChannelTransport::new(to_phone_tx, from_phone_rx));

        let request_channels = Arc::new(Mutex::new(std::collections::HashMap::new()));

        // Start the central dispatcher thread
        dispatch_messages(transport.clone(), request_channels.clone());

        let mut bluetooth = Self {
            inner: Arc::new(BluetoothInner {
                transport,
                request_timeout,
                next_request_id: AtomicU32::new(1),
                request_channels,
            }),
            is_connected: Arc::new(Mutex::new(false)),
            game_load_tx: Arc::new(Mutex::new(game_load_tx)),
        };

        let ble_runtime = bluetooth.setup_runtime(device_name, to_phone_rx, from_phone_tx);

        ble_runtime
            .map(|runtime| {
                runtime.spawn();
            })
            .unwrap_or_else(|e| {
                error!("Failed to setup BLE runtime: {:?}", e);
            });

        bluetooth
    }

    fn setup_runtime(
        &mut self,
        device_name: &str,
        to_phone_rx: Receiver<BoardToPhone>,
        from_phone_tx: Sender<PhoneToBoard>,
    ) -> Result<BleRuntime, BluetoothError> {
        self.is_connected = Arc::new(Mutex::new(false));

        let ble_device = BLEDevice::take();
        let ble_advertiser = ble_device.get_advertising();
        let server = ble_device.get_server();

        {
            let connection_flag = Arc::clone(&self.is_connected);
            server.on_connect(move |server, desc| {
                info!("BLE client connected: {:?}", desc);
                if let Err(e) = server.update_conn_params(desc.conn_handle(), 24, 48, 0, 60) {
                    warn!("Failed to update connection params: {:?}", e);
                }
                *connection_flag.lock().unwrap() = true;
            });
        }

        let (
            bridge_request_characteristic,
            bridge_response_characteristic,
            game_key_characteristic,
        ) = {
            let service = server.create_service(uuid128!(SERVICE_UUID));
            // Request characteristic: board -> phone notifications only.
            let bridge_request_characteristic = service.lock().create_characteristic(
                uuid128!(BRIDGE_REQUEST_CHARACTERISTIC_UUID),
                NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
            );

            // Response characteristic: phone -> board writes only.
            let bridge_response_characteristic = service.lock().create_characteristic(
                uuid128!(BRIDGE_RESPONSE_CHARACTERISTIC_UUID),
                NimbleProperties::READ | NimbleProperties::WRITE,
            );

            // Game key characteristic: phone -> board game id
            let game_key_characteristic = service.lock().create_characteristic(
                uuid128!(GAME_KEY_CHARACTERISTIC_UUID),
                NimbleProperties::READ | NimbleProperties::WRITE,
            );

            (
                bridge_request_characteristic,
                bridge_response_characteristic,
                game_key_characteristic,
            )
        };

        {
            let chr = bridge_request_characteristic.clone();
            let connection_flag = Arc::clone(&self.is_connected);
            server.on_disconnect(move |_desc, _reason| {
                info!("BLE disconnected, restarting advertising");
                let _ = ble_advertiser.lock().start();
                let _ = chr.lock().set_value(b"");
                *connection_flag.lock().unwrap() = false;
            });
        }

        {
            let tx = from_phone_tx.clone();
            let rx_buffer = Arc::new(Mutex::new(Vec::new()));
            let characteristic = bridge_response_characteristic.clone();
            info!("start listening on rx characteristic changes");
            characteristic.lock().on_write(move |args| {
                let data = args.recv_data();

                const MAX_MULTI_FRAME_LEN: usize = 4096;

                let mut buffer = rx_buffer.lock().unwrap();

                if buffer.len() + data.len() > MAX_MULTI_FRAME_LEN {
                    warn!(
                        "Incoming BLE data exceeded max frame length ({}), clearing buffer",
                        MAX_MULTI_FRAME_LEN
                    );
                    buffer.clear();
                    if data.len() > MAX_MULTI_FRAME_LEN {
                        warn!("Single BLE write too large, dropping");
                        return;
                    }
                }

                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    match decode_frame(&frame) {
                        Ok(msg) => {
                            if let Err(e) = tx.send(msg) {
                                error!("Failed to queue incoming BLE frame: {:?}", e);
                            }
                        }
                        Err(e) => warn!("Failed to decode incoming BLE frame: {:?}", e),
                    }
                }
            });
        }

        {
            let event_tx = Arc::clone(&self.game_load_tx);
            let characteristic = game_key_characteristic.clone();
            let game_key_buffer = Arc::new(Mutex::new(Vec::new()));
            characteristic.lock().on_write(move |args| {
                // Receive incoming chunk
                let data = args.recv_data();

                // Decode complete lines, buffer is updated in-place by the utility
                // Combine with any previously stored partial bytes and decode raw frames
                let mut buf_guard = game_key_buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buf_guard);

                // Forward each complete raw frame as a LoadNewGame event (decode UTF-8)
                for frame in frames {
                    // Try to convert to UTF-8; fall back to lossy replacement if invalid
                    let maybe_str = match String::from_utf8(frame) {
                        Ok(s) => s,
                        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                    };
                    let game_id = maybe_str.trim();
                    if game_id.is_empty() {
                        warn!("Received empty game id over BLE load characteristic");
                        continue;
                    }
                    let sender = event_tx.lock().unwrap().clone();
                    if let Err(e) = sender.send(Event::GameCommand(GameCommandEvent::LoadNewGame(
                        game_id.to_string(),
                    ))) {
                        warn!("Failed to forward BLE game id: {:?}", e);
                    } else {
                        info!("Forwarded BLE game id: {}", game_id);
                    }
                }
            });
        }

        ble_advertiser
            .lock()
            .set_data(
                BLEAdvertisementData::new()
                    .name(device_name)
                    .add_service_uuid(uuid128!(SERVICE_UUID)),
            )
            .map_err(|e| BluetoothError::Transport(e.to_string()))?;

        ble_advertiser
            .lock()
            .start()
            .map_err(|e| BluetoothError::Transport(e.to_string()))?;

        Ok(BleRuntime {
            outgoing_rx: to_phone_rx,
            tx_characteristic: bridge_request_characteristic,
        })
    }

    fn next_id(&self) -> u32 {
        self.inner.next_request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Register a channel for a request ID and wait for a response
    fn await_response_body(&self, id: u32) -> Result<String, BluetoothError> {
        let (tx, rx) = std::sync::mpsc::channel();

        // Register the channel for this request
        {
            let mut channels = self.inner.request_channels.lock().unwrap();
            channels.insert(id, tx);
        }

        let deadline = Instant::now() + self.inner.request_timeout;

        loop {
            let now = Instant::now();
            if now >= deadline {
                // Cleanup: remove the channel
                self.inner.request_channels.lock().unwrap().remove(&id);
                return Err(BluetoothError::Timeout);
            }

            let timeout = deadline.saturating_duration_since(now);
            match rx.recv_timeout(timeout) {
                Ok(PhoneToBoard::Response { id: resp_id, body }) if resp_id == id => {
                    // Cleanup: remove the channel
                    self.inner.request_channels.lock().unwrap().remove(&id);
                    return Ok(body);
                }
                Ok(PhoneToBoard::StreamClosed { id: resp_id }) if resp_id == id => {
                    // Cleanup: remove the channel
                    self.inner.request_channels.lock().unwrap().remove(&id);
                    return Ok(String::new());
                }
                Ok(msg) => {
                    warn!("Unexpected message for request {}: {:?}", id, msg);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Cleanup: remove the channel
                    self.inner.request_channels.lock().unwrap().remove(&id);
                    return Err(BluetoothError::Timeout);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Cleanup: remove the channel
                    self.inner.request_channels.lock().unwrap().remove(&id);
                    return Err(BluetoothError::Transport("channel disconnected".into()));
                }
            }
        }
    }

    fn push_chunk(tx: &Sender<String>, buffer: &mut String, chunk: &str) {
        buffer.push_str(chunk);
        while let Some(pos) = buffer.find('\n') {
            let line = buffer.drain(..=pos).collect::<String>();
            let cleaned = line.trim();
            if !cleaned.is_empty() {
                let _ = tx.send(cleaned.to_string());
            }
        }
    }

    /// Stream handler thread - reads from a dedicated channel and processes stream data
    fn handle_stream(rx: Receiver<PhoneToBoard>, id: u32, tx: Sender<String>) {
        let mut buffer = String::new();

        loop {
            match rx.recv() {
                Ok(PhoneToBoard::StreamData { id: msg_id, chunk }) if msg_id == id => {
                    info!(
                        "handle_stream: received StreamData for id {}, chunk: {:?}",
                        msg_id, chunk
                    );
                    Bluetooth::push_chunk(&tx, &mut buffer, &chunk);
                }
                Ok(PhoneToBoard::StreamClosed { id: msg_id }) if msg_id == id => {
                    info!("handle_stream: stream closed for id {}", msg_id);
                    break;
                }
                Ok(msg) => {
                    warn!("handle_stream: unexpected message: {:?}", msg);
                }
                Err(e) => {
                    info!("handle_stream: channel closed, exiting: {:?}", e);
                    break;
                }
            }
        }
    }
}

impl Requester for Bluetooth {
    type RequestError = BluetoothError;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), BluetoothError> {
        let id = self.next_id();

        info!("stream: starting stream with id {} for url {}", id, url);

        // Create a channel for this stream
        let (stream_tx, stream_rx) = std::sync::mpsc::channel();

        // Register the channel
        {
            let mut channels = self.inner.request_channels.lock().unwrap();
            channels.insert(id, stream_tx);
            info!("stream: registered channel for id {}", id);
        }

        // Send the stream request
        self.inner.transport.send(BoardToPhone::Request {
            id,
            method: RequestMethod::Stream,
            url: url.to_string(),
            body: None,
        })?;

        info!("stream: sent request for id {}", id);

        // Spawn the stream handler thread
        let tx_clone = tx.clone();
        thread::spawn(move || {
            info!("stream handler thread started for id {}", id);
            Bluetooth::handle_stream(stream_rx, id, tx_clone);
            info!("stream handler thread exited for id {}", id);
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, BluetoothError> {
        let id = self.next_id();

        self.inner.transport.send(BoardToPhone::Request {
            id,
            method: RequestMethod::Post,
            url: url.to_string(),
            body: Some(body.to_string()),
        })?;

        self.await_response_body(id)
    }

    fn get(&self, url: &str) -> Result<String, BluetoothError> {
        let id = self.next_id();

        self.inner.transport.send(BoardToPhone::Request {
            id,
            method: RequestMethod::Get,
            url: url.to_string(),
            body: None,
        })?;

        self.await_response_body(id)
    }

    fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }
}

/// BLE transport that owns the NimBLE TX characteristic and bridges the connector
/// channel to outbound GATT notifications.
pub struct BleRuntime {
    outgoing_rx: Receiver<BoardToPhone>,
    tx_characteristic: Arc<esp32_nimble::utilities::mutex::Mutex<BLECharacteristic>>,
}

impl BleRuntime {
    /// Spawn a background bridge that forwards BoardToPhone frames over the TX
    /// characteristic as BLE notifications. Returns the thread handle in case
    /// the caller wants to join/monitor.
    pub fn spawn(self) -> JoinHandle<()> {
        std::thread::spawn(move || {
            while let Ok(msg) = self.outgoing_rx.recv() {
                match encode_frame(&msg) {
                    Ok(frame) => {
                        let mut chr = self.tx_characteristic.lock();
                        for chunk in frame.chunks(MIN_MTU_PAYLOAD) {
                            chr.set_value(chunk);
                            chr.notify();
                        }
                    }
                    Err(e) => warn!("Failed to encode frame: {:?}", e),
                }
            }
        })
    }
}
