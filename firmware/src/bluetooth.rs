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

use chess_game::requester::Requester;
use esp32_nimble::{uuid128, BLEAdvertisementData, BLECharacteristic, BLEDevice, NimbleProperties};
use log::*;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;
pub const SERVICE_UUID: &str = "b4d75b6c-7284-4268-8621-6e3cef3c6ac4";
pub const DATA_TX_CHAR_UUID: &str = "aa8381af-049a-46c2-9c92-1db7bd28883c";
pub const DATA_RX_CHAR_UUID: &str = "29e463e6-a210-4234-8d1d-4daf345b41de";

// TODO: can I increase the MTU?
// Keep notifications within the lowest possible BLE ATT MTU (20 bytes -> 23 byte payload).
const MIN_MTU_PAYLOAD: usize = 20;

#[derive(Debug)]
pub enum BluetoothError {
    Transport(String),
    Timeout,
    Protocol(String),
    Remote(String),
}

impl std::fmt::Display for BluetoothError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BluetoothError::Transport(msg) => write!(f, "transport error: {}", msg),
            BluetoothError::Timeout => write!(f, "timeout waiting for response"),
            BluetoothError::Protocol(msg) => write!(f, "protocol error: {}", msg),
            BluetoothError::Remote(msg) => write!(f, "remote error: {}", msg),
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
    Ping {
        id: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhoneToBoard {
    Response { id: u32, body: String },
    StreamData { id: u32, chunk: String },
    StreamClosed { id: u32 },
    Pong { id: u32 },
    Error { id: Option<u32>, message: String },
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

    info!("payload: {:?}", str::from_utf8(payload));

    serde_json::from_slice::<Frame<PhoneToBoard>>(&without_newline)
        .map(|frame| frame.msg)
        .map_err(|e| BluetoothError::Protocol(e.to_string()))
}

pub trait Transport: Send + Sync {
    fn send(&self, msg: BoardToPhone) -> Result<(), BluetoothError>;
    fn recv(&self, timeout: Duration) -> Result<Option<PhoneToBoard>, BluetoothError>;
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

    fn recv(&self, timeout: Duration) -> Result<Option<PhoneToBoard>, BluetoothError> {
        match self.to_board.lock().unwrap().recv_timeout(timeout) {
            Ok(msg) => Ok(Some(msg)),
            Err(RecvTimeoutError::Disconnected) => {
                Err(BluetoothError::Transport("ble link closed".into()))
            }
            Err(RecvTimeoutError::Timeout) => Ok(None),
        }
    }
}

struct BluetoothInner {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
    next_request_id: AtomicU32,
    pending: Mutex<Vec<PhoneToBoard>>,
}

#[derive(Clone)]
pub struct Bluetooth {
    inner: Arc<BluetoothInner>,
    is_connected: Arc<Mutex<bool>>,
}

impl Bluetooth {
    pub fn create_and_spawn(device_name: &str, request_timeout: Duration) -> Self {
        let (to_phone_tx, to_phone_rx) = std::sync::mpsc::channel();
        let (from_phone_tx, from_phone_rx) = std::sync::mpsc::channel();
        let transport = Arc::new(ChannelTransport::new(to_phone_tx, from_phone_rx));

        let mut bluetooth = Self {
            inner: Arc::new(BluetoothInner {
                transport,
                request_timeout,
                next_request_id: AtomicU32::new(1),
                pending: Mutex::new(Vec::new()),
            }),
            is_connected: Arc::new(Mutex::new(false)),
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

        let (tx_characteristic, rx_characteristic) = {
            let service = server.create_service(uuid128!(SERVICE_UUID));
            // TX characteristic: board -> phone notifications only.
            let tx_chr = service.lock().create_characteristic(
                uuid128!(DATA_TX_CHAR_UUID),
                NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
            );

            // RX characteristic: phone -> board writes only.
            let rx_chr = service.lock().create_characteristic(
                uuid128!(DATA_RX_CHAR_UUID),
                NimbleProperties::READ | NimbleProperties::WRITE,
            );

            (tx_chr, rx_chr)
        };

        {
            let chr = tx_characteristic.clone();
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
            let chr = rx_characteristic.clone();
            chr.lock().on_write(move |args| {
                let data = args.recv_data();
                info!("frame received {:?}", data);
                let mut buffer = rx_buffer.lock().unwrap();

                const MAX_MULTI_FRAME_LEN: usize = 4096;

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

                buffer.extend_from_slice(data);

                while let Some(pos) = buffer.iter().position(|b| *b == b'\n' || *b == b'\r') {
                    let frame: Vec<u8> = buffer.drain(..=pos).collect();
                    match decode_frame(&frame) {
                        Ok(msg) => {
                            if let Err(e) = tx.send(msg) {
                                error!("Failed to queue incoming BLE frame: {:?}", e);
                            }
                        }
                        Err(e) => warn!("Failed to decode incoing BLE frame: {:?}", e),
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
            tx_characteristic,
        })
    }

    fn next_id(&self) -> u32 {
        self.inner.next_request_id.fetch_add(1, Ordering::SeqCst)
    }

    fn stash_message(&self, msg: PhoneToBoard) {
        self.inner.pending.lock().unwrap().push(msg);
    }

    fn recv_with_pending(&self, timeout: Duration) -> Result<Option<PhoneToBoard>, BluetoothError> {
        if let Some(msg) = self.inner.pending.lock().unwrap().pop() {
            return Ok(Some(msg));
        }

        self.inner.transport.recv(timeout)
    }

    fn await_response_body(&self, id: u32) -> Result<String, BluetoothError> {
        let deadline = Instant::now() + self.inner.request_timeout;

        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(BluetoothError::Timeout);
            }

            let timeout = deadline.saturating_duration_since(now);
            match self.recv_with_pending(timeout)? {
                Some(PhoneToBoard::Response { id: resp_id, body }) if resp_id == id => {
                    return Ok(body)
                }
                Some(PhoneToBoard::StreamClosed { id: resp_id }) if resp_id == id => {
                    return Ok(String::new());
                }
                Some(PhoneToBoard::Error {
                    id: Some(err_id),
                    message,
                }) if err_id == id || err_id == 0 => {
                    return Err(BluetoothError::Remote(message));
                }
                Some(PhoneToBoard::Error { id: None, message }) => {
                    return Err(BluetoothError::Remote(message));
                }
                Some(msg) => self.stash_message(msg),
                None => return Err(BluetoothError::Timeout),
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

    fn handle_stream(
        inner: Arc<BluetoothInner>,
        id: u32,
        tx: Sender<String>,
        initial_chunk: Option<String>,
    ) {
        let mut buffer = String::new();
        if let Some(chunk) = initial_chunk {
            Bluetooth::push_chunk(&tx, &mut buffer, &chunk);
        }

        loop {
            match {
                if let Some(msg) = inner.pending.lock().unwrap().pop() {
                    Ok(Some(msg))
                } else {
                    inner.transport.recv(Duration::from_millis(500))
                }
            } {
                Ok(Some(PhoneToBoard::StreamData { id: msg_id, chunk })) if msg_id == id => {
                    Bluetooth::push_chunk(&tx, &mut buffer, &chunk);
                }
                Ok(Some(PhoneToBoard::StreamClosed { id: msg_id })) if msg_id == id => break,
                Ok(Some(PhoneToBoard::Error {
                    id: Some(err_id),
                    message,
                })) if err_id == id => {
                    let _ = tx.send(format!("Error: {}", message));
                    break;
                }
                Ok(Some(PhoneToBoard::Error { id: None, message })) => {
                    let _ = tx.send(format!("Error: {}", message));
                    break;
                }
                Ok(Some(other)) => {
                    inner.pending.lock().unwrap().push(other);
                }
                Ok(None) => continue,
                Err(e) => {
                    error!("Stream error: {:?}", e);
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

        self.inner.transport.send(BoardToPhone::Request {
            id,
            method: RequestMethod::Stream,
            url: url.to_string(),
            body: None,
        })?;

        let deadline = Instant::now() + self.inner.request_timeout;
        let initial_chunk: Option<String>;

        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(BluetoothError::Timeout);
            }

            let timeout = deadline.saturating_duration_since(now);
            match self.recv_with_pending(timeout)? {
                Some(PhoneToBoard::StreamData { id: msg_id, chunk }) if msg_id == id => {
                    initial_chunk = Some(chunk);
                    break;
                }
                Some(PhoneToBoard::Response { id: msg_id, body }) if msg_id == id => {
                    initial_chunk = Some(body);
                    break;
                }
                Some(PhoneToBoard::StreamClosed { id: msg_id }) if msg_id == id => {
                    return Ok(());
                }
                Some(PhoneToBoard::Error {
                    id: Some(err_id),
                    message,
                }) if err_id == id || err_id == 0 => {
                    return Err(BluetoothError::Remote(message));
                }
                Some(PhoneToBoard::Error { id: None, message }) => {
                    return Err(BluetoothError::Remote(message));
                }
                Some(msg) => self.stash_message(msg),
                None => return Err(BluetoothError::Timeout),
            }
        }

        let tx_clone = tx.clone();
        let inner = self.inner.clone();

        thread::spawn(move || Bluetooth::handle_stream(inner, id, tx_clone, initial_chunk));

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
                            info!("notify characteristic chunk ({} bytes)", chunk.len());
                            chr.notify();
                        }
                    }
                    Err(e) => warn!("Failed to encode frame: {:?}", e),
                }
            }
        })
    }
}
