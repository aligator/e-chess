use std::{
    str::FromStr,
    sync::{
        mpsc::{Receiver, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::Result;
use chess::{Board, ChessMove, Game};
use chess_game::chess_connector::{ChessConnector, ChessConnectorError, OngoingGame};
use esp32_nimble::{uuid128, BLEAdvertisementData, BLECharacteristic, BLEDevice, NimbleProperties};
use log::*;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;
pub const SERVICE_UUID: &str = "b4d75b6c-7284-4268-8621-6e3cef3c6ac4";
pub const DATA_CHAR_UUID: &str = "80580a69-122f-41a8-88c2-8a355fdba6a8";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoardToPhone {
    /// Request the list of current Lichess games.
    ListGames,
    /// Ask Android to stream a specific game; Android answers with GameLoaded
    /// and then pushes GameState events.
    LoadGame { id: String },
    /// Send a move done on the physical board. Android forwards to Lichess.
    MakeMove { uci: String },
    /// Keep-alive so the phone knows the board is still here.
    Ping { id: u32 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PhoneToBoard {
    Pong {
        id: u32,
    },
    /// Full list of available games.
    GameList {
        games: Vec<OngoingGame>,
    },
    /// Initial snapshot of a game (FEN + moves).
    GameLoaded(GameSnapshot),
    /// Confirmation that a move was forwarded to Lichess.
    MoveApplied {
        ok: bool,
        message: Option<String>,
    },
    /// Upstream game change (new move / take-back requests).
    GameState(GameStatePayload),
    /// General-purpose error.
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Frame<T> {
    pub v: u8,
    #[serde(flatten)]
    pub msg: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSnapshot {
    pub initial_fen: String,
    pub moves: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameStatePayload {
    pub moves: Vec<String>,
    pub white_request_take_back: bool,
    pub black_request_take_back: bool,
}

fn game_from_snapshot(
    snapshot: &GameSnapshot,
) -> Result<Game, chess_game::chess_connector::ChessConnectorError> {
    let board = Board::from_str(&snapshot.initial_fen)
        .map_err(|e| chess_game::chess_connector::ChessConnectorError::InvalidFen(e.to_string()))?;

    for chess_move in &snapshot.moves {
        let chess_move = ChessMove::from_san(&board, chess_move).map_err(|err| {
            chess_game::chess_connector::ChessConnectorError::InvalidResponse(format!(
                "invalid UCI move: {}, {:?}",
                chess_move, err
            ))
        })?;
        let mut result = Board::default();
        board.make_move(chess_move, &mut result); // Sets the board value to result.
    }

    Ok(Game::new_with_board(board.clone()))
}

pub fn encode_frame(msg: &BoardToPhone) -> Result<Vec<u8>, ChessConnectorError> {
    serde_json::to_string(&Frame {
        v: PROTOCOL_VERSION,
        msg: msg.clone(),
    })
    .map(|mut body| {
        body.push('\n'); // newline delimits frames
        body.into_bytes()
    })
    .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))
}

pub fn decode_frame(payload: &[u8]) -> Result<PhoneToBoard, ChessConnectorError> {
    let without_newline = payload
        .iter()
        .copied()
        .take_while(|b| *b != b'\n' && *b != b'\r')
        .collect::<Vec<u8>>();

    info!("payload: {:?}", str::from_utf8(payload));

    serde_json::from_slice::<Frame<PhoneToBoard>>(&without_newline)
        .map(|frame| frame.msg)
        .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))
}

pub trait Transport: Send + Sync {
    fn send(&self, msg: BoardToPhone) -> Result<(), ChessConnectorError>;
    fn recv(&self, timeout: Duration) -> Result<Option<PhoneToBoard>, ChessConnectorError>;
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
    fn send(&self, msg: BoardToPhone) -> Result<(), ChessConnectorError> {
        self.from_board
            .send(msg)
            .map_err(|e| ChessConnectorError::RequestError(e.to_string()))
    }

    fn recv(&self, timeout: Duration) -> Result<Option<PhoneToBoard>, ChessConnectorError> {
        match self.to_board.lock().unwrap().recv_timeout(timeout) {
            Ok(msg) => Ok(Some(msg)),
            Err(RecvTimeoutError::Disconnected) => {
                Err(ChessConnectorError::RequestError("ble link closed".into()))
            }
            Err(RecvTimeoutError::Timeout) => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct Bluetooth {
    transport: Arc<dyn Transport>,
    request_timeout: Duration,
}

impl Bluetooth {
    fn new(transport: Arc<dyn Transport>, request_timeout: Duration) -> Self {
        Self {
            transport,
            request_timeout,
        }
    }

    /// Helper to wire into the BLE stack: you get a connector plus the channels
    /// you can bridge to the NimBLE callbacks (board -> phone notifications and
    /// incoming phone -> board writes).
    pub fn with_channels(
        request_timeout: Duration,
    ) -> (Self, Receiver<BoardToPhone>, Sender<PhoneToBoard>) {
        let (to_phone_tx, to_phone_rx) = std::sync::mpsc::channel();
        let (from_phone_tx, from_phone_rx) = std::sync::mpsc::channel();

        (
            Self::new(
                Arc::new(ChannelTransport::new(to_phone_tx, from_phone_rx)),
                request_timeout,
            ),
            to_phone_rx,
            from_phone_tx,
        )
    }

    fn request(&self, msg: BoardToPhone) -> Result<PhoneToBoard, ChessConnectorError> {
        self.transport.send(msg)?;
        self.transport
            .recv(self.request_timeout)?
            .ok_or_else(|| ChessConnectorError::RequestError("timeout waiting for response".into()))
    }
}

impl ChessConnector for Bluetooth {
    fn find_open_games(&self) -> Result<Vec<OngoingGame>, ChessConnectorError> {
        match self.request(BoardToPhone::ListGames)? {
            PhoneToBoard::GameList { games } => Ok(games),
            PhoneToBoard::Error { message } => Err(ChessConnectorError::RequestError(message)),
            other => Err(ChessConnectorError::InvalidResponse(format!(
                "unexpected response to ListGames: {:?}",
                other
            ))),
        }
    }

    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError> {
        match self.request(BoardToPhone::LoadGame { id: id.to_string() })? {
            PhoneToBoard::GameLoaded(snapshot) => {
                let game = game_from_snapshot(&snapshot)
                    .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;
                Ok(game)
            }
            PhoneToBoard::Error { message } => Err(ChessConnectorError::RequestError(message)),
            other => Err(ChessConnectorError::InvalidResponse(format!(
                "unexpected response to LoadGame: {:?}",
                other
            ))),
        }
    }

    fn make_move(&self, chess_move: ChessMove) -> bool {
        match self.request(BoardToPhone::MakeMove {
            uci: chess_move.to_string(),
        }) {
            Ok(PhoneToBoard::MoveApplied { ok, .. }) => ok,
            Ok(PhoneToBoard::Error { message }) => {
                error!("Error applying move: {}", message);
                false
            }
            Ok(other) => {
                error!("Unexpected response to MakeMove: {:?}", other);
                false
            }
            Err(e) => {
                error!("Request error applying move: {}", e);
                false
            }
        }
    }

    fn next_event(
        &self,
    ) -> Result<Option<chess_game::chess_connector::GameEvent>, ChessConnectorError> {
        match self.transport.recv(Duration::from_millis(0))? {
            Some(PhoneToBoard::GameState(payload)) => {
                Ok(Some(chess_game::chess_connector::GameEvent::State(
                    chess_game::chess_connector::GameState {
                        white_request_take_back: payload.white_request_take_back,
                        black_request_take_back: payload.black_request_take_back,
                        moves: payload.moves,
                    },
                )))
            }
            Some(other) => {
                warn!("Ignoring unexpected message from phone: {:?}", other);
                Ok(None)
            }
            None => Ok(None),
        }
    }
}

/// BLE transport that owns the NimBLE characteristic and bridges the connector
/// channels to GATT notifications/writes.
pub struct BleRuntime {
    outgoing_rx: Receiver<BoardToPhone>,
    characteristic: Arc<esp32_nimble::utilities::mutex::Mutex<BLECharacteristic>>,
}

impl BleRuntime {
    /// Spawn a background bridge that forwards BoardToPhone frames as BLE
    /// notifications. Returns the thread handle in case the caller wants to
    /// join/monitor.
    pub fn spawn(self) -> JoinHandle<()> {
        std::thread::spawn(move || {
            while let Ok(msg) = self.outgoing_rx.recv() {
                match encode_frame(&msg) {
                    Ok(frame) => {
                        let mut chr = self.characteristic.lock();
                        chr.set_value(&frame);
                        let _ = chr.notify();
                    }
                    Err(e) => warn!("Failed to encode frame: {:?}", e),
                }
            }
        })
    }
}

pub fn init_ble_server(
    device_name: &str,
    request_timeout: Duration,
) -> Result<(Bluetooth, BleRuntime)> {
    let (connector, to_phone_rx, from_phone_tx) = Bluetooth::with_channels(request_timeout);

    let ble_device = BLEDevice::take();
    let ble_advertiser = ble_device.get_advertising();
    let server = ble_device.get_server();

    server.on_connect(|server, desc| {
        info!("BLE client connected: {:?}", desc);
        if let Err(e) = server.update_conn_params(desc.conn_handle(), 24, 48, 0, 60) {
            warn!("Failed to update connection params: {:?}", e);
        }
    });

    let characteristic = {
        let service = server.create_service(uuid128!(SERVICE_UUID));
        let chr = service.lock().create_characteristic(
            uuid128!(DATA_CHAR_UUID),
            NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
        );

        chr
    };

    {
        let chr = characteristic.clone();
        server.on_disconnect(move |_desc, _reason| {
            info!("BLE disconnected, restarting advertising");
            let _ = ble_advertiser.lock().start();
            let _ = chr.lock().set_value(b"");
        });
    }

    {
        let tx = from_phone_tx.clone();
        let rx_buffer = Arc::new(Mutex::new(Vec::new()));
        let chr = characteristic.clone();
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
        .map_err(|e| ChessConnectorError::RequestError(e.to_string()))?;

    ble_advertiser
        .lock()
        .start()
        .map_err(|e| ChessConnectorError::RequestError(e.to_string()))?;

    let runtime = BleRuntime {
        outgoing_rx: to_phone_rx,
        characteristic: characteristic,
    };

    Ok((connector, runtime))
}
