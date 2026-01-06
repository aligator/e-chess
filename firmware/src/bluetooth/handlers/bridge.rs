//! HTTP Bridge Handler
//!
//! Implements a protocol over BLE GATT characteristics to send HTTP-like
//! requests from the chess board to a connected client, which performs
//! the actual network requests and streams data back to the board.

use crate::bluetooth::{types::*, util::*};
use chess_game::requester::Requester;
use esp32_nimble::{
    utilities::mutex::Mutex as NimbleMutex, uuid128, BLECharacteristic, BLEService,
    NimbleProperties,
};
use log::*;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub const BRIDGE_REQUEST_CHARACTERISTIC_UUID: &str = "aa8381af-049a-46c2-9c92-1db7bd28883c";
pub const BRIDGE_RESPONSE_CHARACTERISTIC_UUID: &str = "29e463e6-a210-4234-8d1d-4daf345b41de";

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

/// Channel-based transport for the bridge protocol
pub trait Transport: Send + Sync {
    fn send(&self, msg: BridgeRequest) -> Result<()>;
    fn recv(&self) -> Result<BridgeResponse>;
}

/// Simple channel-based transport implementation
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
    fn send(&self, msg: BridgeRequest) -> Result<()> {
        info!("{:?}", msg);
        self.from_board
            .send(msg)
            .map_err(|e| BluetoothError::Transport(e.to_string()))
    }

    fn recv(&self) -> Result<BridgeResponse> {
        self.to_board
            .lock()
            .unwrap()
            .recv()
            .map_err(|_| BluetoothError::Transport("ble link closed".into()))
    }
}

/// Bridge handler that manages HTTP-like requests over BLE
pub struct BridgeHandler {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
    next_request_id: AtomicU32,
    request_channels: Arc<Mutex<std::collections::HashMap<u32, Sender<BridgeResponse>>>>,
    is_connected: Arc<Mutex<bool>>, // Shared with BluetoothService
}

impl BridgeHandler {
    /// Create a new bridge handler with shared connection state
    pub fn new(
        request_timeout: Duration,
        is_connected: Arc<Mutex<bool>>,
    ) -> (Self, Receiver<BridgeRequest>, Sender<BridgeResponse>) {
        let (to_phone_tx, to_phone_rx) = std::sync::mpsc::channel();
        let (from_phone_tx, from_phone_rx) = std::sync::mpsc::channel();
        let transport = Arc::new(ChannelTransport::new(to_phone_tx, from_phone_rx));

        let handler = Self {
            transport,
            request_timeout,
            next_request_id: AtomicU32::new(1),
            request_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            is_connected,
        };

        (handler, to_phone_rx, from_phone_tx)
    }

    /// Register bridge characteristics with the BLE service
    ///
    /// Parameters:
    /// - bridge_response_tx: Send decoded responses from phone to dispatcher
    /// - bridge_request_rx: Receive requests to send to phone
    pub fn register_characteristics(
        &self,
        service: &Arc<NimbleMutex<BLEService>>,
        bridge_response_tx: Sender<BridgeResponse>,
        bridge_request_rx: Receiver<BridgeRequest>,
    ) -> Result<Arc<NimbleMutex<BLECharacteristic>>> {
        // Request characteristic: board -> phone notifications
        let request_characteristic = service.lock().create_characteristic(
            uuid128!(BRIDGE_REQUEST_CHARACTERISTIC_UUID),
            NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
        );

        // Response characteristic: phone -> board writes
        let response_characteristic = service.lock().create_characteristic(
            uuid128!(BRIDGE_RESPONSE_CHARACTERISTIC_UUID),
            NimbleProperties::WRITE,
        );

        // Setup response write handler
        {
            let buffer = Arc::new(Mutex::new(Vec::new()));
            response_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();
                let mut buffer = buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    match decode_response_frame(&frame) {
                        Ok(msg) => {
                            if let Err(e) = bridge_response_tx.send(msg) {
                                error!("Failed to queue incoming BLE frame: {:?}", e);
                            }
                        }
                        Err(e) => warn!("Failed to decode incoming bridge BLE frame: {:?}", e),
                    }
                }
            });
        }

        // Start thread to forward requests to BLE (board -> phone)
        let request_char_clone = request_characteristic.clone();
        thread::spawn(move || {
            info!("Bridge request sender thread started");
            while let Ok(msg) = bridge_request_rx.recv() {
                match encode_json_frame(&msg) {
                    Ok(frame) => {
                        send_chunked_notification(&request_char_clone, &frame);
                    }
                    Err(e) => {
                        warn!("Failed to encode bridge request: {:?}", e);
                    }
                }
            }
            info!("Bridge request sender thread exiting");
        });

        Ok(request_characteristic)
    }

    /// Start the dispatcher thread that routes responses to request handlers
    pub fn start_dispatcher(&self) {
        let transport = self.transport.clone();
        let request_channels = self.request_channels.clone();

        thread::spawn(move || {
            info!("Bridge dispatcher thread started");
            loop {
                match transport.recv() {
                    Ok(msg) => {
                        let id = match &msg {
                            BridgeResponse::Response { id, .. } => *id,
                            BridgeResponse::StreamData { id, .. } => *id,
                            BridgeResponse::StreamClosed { id } => *id,
                        };

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
            info!("Bridge dispatcher thread exiting");
        });
    }

    fn next_id(&self) -> u32 {
        self.next_request_id.fetch_add(1, Ordering::SeqCst)
    }

    fn await_response_body(&self, id: u32) -> Result<String> {
        let (tx, rx) = std::sync::mpsc::channel();

        {
            let mut channels = self.request_channels.lock().unwrap();
            channels.insert(id, tx);
        }

        let deadline = Instant::now() + self.request_timeout;

        loop {
            let now = Instant::now();
            if now >= deadline {
                self.request_channels.lock().unwrap().remove(&id);
                return Err(BluetoothError::Timeout);
            }

            let timeout = deadline.saturating_duration_since(now);
            match rx.recv_timeout(timeout) {
                Ok(BridgeResponse::Response { id: resp_id, body }) if resp_id == id => {
                    self.request_channels.lock().unwrap().remove(&id);
                    return Ok(body);
                }
                Ok(BridgeResponse::StreamClosed { id: resp_id }) if resp_id == id => {
                    self.request_channels.lock().unwrap().remove(&id);
                    return Ok(String::new());
                }
                Ok(msg) => {
                    warn!("Unexpected message for request {}: {:?}", id, msg);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    self.request_channels.lock().unwrap().remove(&id);
                    return Err(BluetoothError::Timeout);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    self.request_channels.lock().unwrap().remove(&id);
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

    fn handle_stream(rx: Receiver<BridgeResponse>, id: u32, tx: Sender<String>) {
        let mut buffer = String::new();

        loop {
            match rx.recv() {
                Ok(BridgeResponse::StreamData { id: msg_id, chunk }) if msg_id == id => {
                    Self::push_chunk(&tx, &mut buffer, &chunk);
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

impl Requester for BridgeHandler {
    type RequestError = BluetoothError;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<()> {
        let id = self.next_id();

        info!("stream: starting stream with id {} for url {}", id, url);

        let (stream_tx, stream_rx) = std::sync::mpsc::channel();

        {
            let mut channels = self.request_channels.lock().unwrap();
            channels.insert(id, stream_tx);
            info!("stream: registered channel for id {}", id);
        }

        self.transport.send(BridgeRequest::Request {
            id,
            method: RequestMethod::Stream,
            url: url.to_string(),
            body: None,
        })?;

        info!("stream: sent request for id {}", id);

        let tx_clone = tx.clone();
        thread::spawn(move || {
            info!("stream handler thread started for id {}", id);
            Self::handle_stream(stream_rx, id, tx_clone);
            info!("stream handler thread exited for id {}", id);
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String> {
        let id = self.next_id();

        self.transport.send(BridgeRequest::Request {
            id,
            method: RequestMethod::Post,
            url: url.to_string(),
            body: Some(body.to_string()),
        })?;

        self.await_response_body(id)
    }

    fn get(&self, url: &str) -> Result<String> {
        let id = self.next_id();

        self.transport.send(BridgeRequest::Request {
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

fn decode_response_frame(payload: &[u8]) -> Result<BridgeResponse> {
    serde_json::from_slice::<Frame<BridgeResponse>>(payload)
        .map(|frame| frame.msg)
        .map_err(|e| BluetoothError::Protocol(e.to_string()))
}
