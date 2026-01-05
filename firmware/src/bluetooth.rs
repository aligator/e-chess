//! Bluetooth LE transport for chess game requester.
//! Implements a protocol over BLE GATT characteristics to send HTTP-like
//! requests from the chess board to a connected client, which performs
//! the actual network requests and streams data back to the board.
//!
//! Implements also some direct communication for the game state.
//! Note that the architecture is designed so that the app does not really know much about lichess.
//! (except the api key - for now)
//! That way all chess logic is only defined on the board side at one place.
//! The app can however query higher level game state information via the game state characteristic.
use std::{
    str,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    event::EventManager,
    game::{GameCommandEvent, GameStateEvent},
    Event,
};
use chess_game::{chess_connector::OngoingGame, requester::Requester};
use esp32_nimble::{uuid128, BLEAdvertisementData, BLECharacteristic, BLEDevice, NimbleProperties};
use log::*;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;
pub const SERVICE_UUID: &str = "b4d75b6c-7284-4268-8621-6e3cef3c6ac4";

// The bridge is only for bridging upstream (e.g. lichess) http calls from the board over ble.
// It should not be used to query data directly.
pub const BRIDGE_REQUEST_CHARACTERISTIC_UUID: &str = "aa8381af-049a-46c2-9c92-1db7bd28883c";
pub const BRIDGE_RESPONSE_CHARACTERISTIC_UUID: &str = "29e463e6-a210-4234-8d1d-4daf345b41de";

// For direct communication extra ble characteristics are used.
/// The action characteristic is used to send commands from the phone to the board.
/// E.g. to start a new game, refresh open games, takeback, ...
pub const ACTION_CHARACTERISTIC_UUID: &str = "0de794de-c3a3-48b8-bd81-893d30342c87";

// The following characteristic is readonly and expose information about the board/game.
/// Get game state events
pub const EVENT_CHARACTERISTIC_UUID: &str = "a1a289ce-d553-4d81-b52d-44e6484507b3";

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
pub enum BridgeRequest {
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
pub enum BridgeResponse {
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

pub trait Transport: Send + Sync {
    fn send(&self, msg: BridgeRequest) -> Result<(), BluetoothError>;
    fn recv(&self) -> Result<BridgeResponse, BluetoothError>;
}

/// Channel-based transport: integrate BLE callbacks by writing decoded
/// `PhoneToBoard` messages into `to_board` and reading `BoardToPhone`
/// notifications from `from_board`.
#[derive(Clone)]
pub struct ChannelTransport {
    from_board: Sender<BridgeRequest>,
    to_board: Arc<Mutex<Receiver<BridgeResponse>>>,
}

impl ChannelTransport {
    pub fn new(from_board: Sender<BridgeRequest>, to_board: Receiver<BridgeResponse>) -> Self {
        Self {
            from_board,
            to_board: Arc::new(Mutex::new(to_board)),
        }
    }
}

impl Transport for ChannelTransport {
    fn send(&self, msg: BridgeRequest) -> Result<(), BluetoothError> {
        info!("{:?}", msg);
        self.from_board
            .send(msg)
            .map_err(|e| BluetoothError::Transport(e.to_string()))
    }

    fn recv(&self) -> Result<BridgeResponse, BluetoothError> {
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
    request_channels: Arc<Mutex<std::collections::HashMap<u32, Sender<BridgeResponse>>>>,
}

#[derive(Clone)]
pub struct Bluetooth {
    inner: Arc<BluetoothInner>,
    is_connected: Arc<Mutex<bool>>,
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
    request_channels: Arc<Mutex<std::collections::HashMap<u32, Sender<BridgeResponse>>>>,
) {
    thread::spawn(move || {
        info!("Dispatcher thread started");
        loop {
            match transport.recv() {
                Ok(msg) => {
                    // Extract the ID from the message
                    let id = match &msg {
                        BridgeResponse::Response { id, .. } => *id,
                        BridgeResponse::StreamData { id, .. } => *id,
                        BridgeResponse::StreamClosed { id } => *id,
                    };

                    // Find the channel for this request ID and send the message
                    let channels = request_channels.lock().unwrap();
                    if let Some(tx) = channels.get(&id) {
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
        event_tx: Sender<Event>,
    ) -> Self {
        info!("create_and_spawn: creating channels");
        let (to_phone_tx, to_phone_rx) = std::sync::mpsc::channel();
        let (from_phone_tx, from_phone_rx) = std::sync::mpsc::channel();
        info!("create_and_spawn: creating transport");
        let transport = Arc::new(ChannelTransport::new(to_phone_tx, from_phone_rx));

        info!("create_and_spawn: creating request_channels");
        let request_channels = Arc::new(Mutex::new(std::collections::HashMap::new()));

        // Start the central dispatcher thread
        info!("create_and_spawn: cloning transport for dispatcher");
        let transport_clone = transport.clone();
        info!("create_and_spawn: cloning request_channels for dispatcher");
        let request_channels_clone = request_channels.clone();
        info!("create_and_spawn: starting dispatcher");
        dispatch_messages(transport_clone, request_channels_clone);

        info!("create_and_spawn: creating Bluetooth struct");
        let mut bluetooth = Self {
            inner: Arc::new(BluetoothInner {
                transport,
                request_timeout,
                next_request_id: AtomicU32::new(1),
                request_channels,
            }),
            is_connected: Arc::new(Mutex::new(false)),
        };
        info!("create_and_spawn: Bluetooth struct created");

        info!("create_and_spawn: calling setup_runtime");
        let ble_runtime =
            bluetooth.setup_runtime(device_name, to_phone_rx, from_phone_tx, event_tx);

        info!("create_and_spawn: setup_runtime completed");
        ble_runtime
            .map(|runtime| {
                info!("create_and_spawn: spawning BLE runtime");
                runtime.spawn();
                info!("create_and_spawn: BLE runtime spawned");
            })
            .unwrap_or_else(|e| {
                error!("Failed to setup BLE runtime: {:?}", e);
            });

        info!("create_and_spawn: returning bluetooth");
        bluetooth
    }

    fn setup_runtime(
        &mut self,
        device_name: &str,
        outgoing_rx: Receiver<BridgeRequest>,
        incoming_tx: Sender<BridgeResponse>,
        event_tx: Sender<Event>,
    ) -> Result<BleRuntime, BluetoothError> {
        info!("setup_runtime: starting");
        self.is_connected = Arc::new(Mutex::new(false));

        info!("setup_runtime: taking BLE device");
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
            action_characteristic,
            // event_characteristic, // TODO: Re-enable
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
                NimbleProperties::WRITE,
            );

            // Action characteristic: send actions from phone to board
            let action_characteristic = service.lock().create_characteristic(
                uuid128!(ACTION_CHARACTERISTIC_UUID),
                NimbleProperties::WRITE,
            );

            // TODO: Re-enable event characteristic once clone() issue is fixed
            // Game state event characteristic: board -> phone notifications for new events
            // let event_characteristic = service.lock().create_characteristic(
            //     uuid128!(EVENT_CHARACTERISTIC_UUID),
            //     NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
            // );

            (
                bridge_request_characteristic,
                bridge_response_characteristic,
                action_characteristic,
                // event_characteristic,
            )
        };

        {
            let characteristic = bridge_request_characteristic.clone();
            let connection_flag = Arc::clone(&self.is_connected);
            server.on_disconnect(move |_desc, _reason| {
                info!("BLE disconnected, restarting advertising");
                let _ = ble_advertiser.lock().start();
                let _ = characteristic.lock().set_value(b"");
                *connection_flag.lock().unwrap() = false;
            });
        }

        // Setup all the on_write handlers
        {
            info!("start listening on incoming http response characteristic");
            let buffer = Arc::new(Mutex::new(Vec::new()));
            pub fn decode_frame(payload: &[u8]) -> Result<BridgeResponse, BluetoothError> {
                serde_json::from_slice::<Frame<BridgeResponse>>(&payload)
                    .map(|frame| frame.msg)
                    .map_err(|e| BluetoothError::Protocol(e.to_string()))
            }
            bridge_response_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();
                let mut buffer = buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    match decode_frame(&frame) {
                        Ok(msg) => {
                            if let Err(e) = incoming_tx.send(msg) {
                                error!("Failed to queue incoming BLE frame: {:?}", e);
                            }
                        }
                        Err(e) => warn!("Failed to decode incoming bridge BLE frame: {:?}", e),
                    }
                }
            });
        }

        {
            info!("setup_runtime: setting up action characteristic with event_tx");
            let buffer = Arc::new(Mutex::new(Vec::new()));
            pub fn decode_frame(payload: &[u8]) -> Result<GameCommandEvent, BluetoothError> {
                serde_json::from_slice::<Frame<GameCommandEvent>>(&payload)
                    .map(|frame| frame.msg)
                    .map_err(|e| BluetoothError::Protocol(e.to_string()))
            }

            action_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();
                let mut buffer = buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    match decode_frame(&frame) {
                        Ok(event) => event_tx
                            .send(Event::GameCommand(event))
                            .unwrap_or_else(|e| {
                                warn!("Failed to forward BLE game command event: {:?}", e)
                            }),
                        Err(e) => warn!("Failed to decode incoming BLE action frame: {:?}", e),
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
            outgoing_rx,
            bridge_request_characteristic,
            // TODO: Re-enable event broadcasting
            // event_rx: event_receiver,
            // event_characteristic: event_characteristic,
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
                Ok(BridgeResponse::Response { id: resp_id, body }) if resp_id == id => {
                    // Cleanup: remove the channel
                    self.inner.request_channels.lock().unwrap().remove(&id);
                    return Ok(body);
                }
                Ok(BridgeResponse::StreamClosed { id: resp_id }) if resp_id == id => {
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
    fn handle_stream(rx: Receiver<BridgeResponse>, id: u32, tx: Sender<String>) {
        let mut buffer = String::new();

        loop {
            match rx.recv() {
                Ok(BridgeResponse::StreamData { id: msg_id, chunk }) if msg_id == id => {
                    Bluetooth::push_chunk(&tx, &mut buffer, &chunk);
                }
                Ok(BridgeResponse::StreamClosed { id: msg_id }) if msg_id == id => {
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
        self.inner.transport.send(BridgeRequest::Request {
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

        self.inner.transport.send(BridgeRequest::Request {
            id,
            method: RequestMethod::Post,
            url: url.to_string(),
            body: Some(body.to_string()),
        })?;

        self.await_response_body(id)
    }

    fn get(&self, url: &str) -> Result<String, BluetoothError> {
        let id = self.next_id();

        self.inner.transport.send(BridgeRequest::Request {
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
    outgoing_rx: Receiver<BridgeRequest>,
    bridge_request_characteristic: Arc<esp32_nimble::utilities::mutex::Mutex<BLECharacteristic>>,
    // TODO: Re-enable event broadcasting once we figure out the clone() issue
    // event_rx: Receiver<Event>,
    // event_characteristic: Arc<esp32_nimble::utilities::mutex::Mutex<BLECharacteristic>>,
}

pub fn encode_json_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, BluetoothError> {
    serde_json::to_string(&Frame {
        v: PROTOCOL_VERSION,
        msg: msg,
    })
    .map(|mut body| {
        body.push('\n');
        body.into_bytes()
    })
    .map_err(|e| BluetoothError::Protocol(e.to_string()))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableGameStateEvent {
    OngoingGamesLoaded { games: Vec<OngoingGame> },
    GameLoaded { game_key: String },
}

impl BleRuntime {
    /// Spawn a background bridge that forwards BoardToPhone frames over the TX
    /// characteristic as BLE notifications. Returns the thread handle in case
    /// the caller wants to join/monitor.
    pub fn spawn(self) -> JoinHandle<()> {
        std::thread::spawn(move || {
            while let Ok(msg) = self.outgoing_rx.recv() {
                match encode_json_frame(&msg) {
                    Ok(frame) => {
                        let mut chr = self.bridge_request_characteristic.lock();
                        for chunk in frame.chunks(MIN_MTU_PAYLOAD) {
                            chr.set_value(chunk);
                            chr.notify();
                        }
                    }
                    Err(e) => warn!("Failed to encode frame: {:?}", e),
                }
            }
        })

        // TODO: Re-enable event broadcasting once clone() issue is fixed
        // let event_join_handle = std::thread::spawn(move || {
        //     while let Ok(event) = self.event_rx.recv() {
        //         let serializeable = match event {
        //             Event::GameState(GameStateEvent::OngoingGamesLoaded(ongoing)) => {
        //                 Some(SerializableGameStateEvent::OngoingGamesLoaded { games: ongoing })
        //             }
        //             Event::GameState(GameStateEvent::GameLoaded(game_key)) => {
        //                 Some(SerializableGameStateEvent::GameLoaded { game_key })
        //             }
        //             _ => None,
        //         };
        //         if serializeable.is_none() {
        //             continue;
        //         }
        //
        //         match encode_json_frame(&serializeable) {
        //             Ok(frame) => {
        //                 let mut chr = self.event_characteristic.lock();
        //                 for chunk in frame.chunks(MIN_MTU_PAYLOAD) {
        //                     chr.set_value(chunk);
        //                     chr.notify();
        //                 }
        //             }
        //             Err(e) => warn!("Failed to encode frame: {:?}", e),
        //         }
        //     }
        // });
        //
        // thread::spawn(move || {
        //     bridge_join_handle.join().unwrap();
        //     event_join_handle.join().unwrap();
        // })
    }
}
