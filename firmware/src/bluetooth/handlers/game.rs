//! Game State Handler
//!
//! Handles direct communication for game state between board and phone.
//! The app can send commands (actions) and receive state updates (events).

use crate::{
    bluetooth::{types::*, util::*},
    game::{GameCommandEvent, GameStateEvent},
    Event,
};
use chess_game::chess_connector::OngoingGame;
use esp32_nimble::{utilities::mutex::Mutex as NimbleMutex, uuid128, BLEService, NimbleProperties};
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

pub const ACTION_CHARACTERISTIC_UUID: &str = "0de794de-c3a3-48b8-bd81-893d30342c87";
pub const EVENT_CHARACTERISTIC_UUID: &str = "a1a289ce-d553-4d81-b52d-44e6484507b3";

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SerializableGameStateEvent {
    OngoingGamesLoaded { games: Vec<OngoingGame> },
    GameLoaded { game_key: String },
}

/// Game handler that manages game state communication over BLE
pub struct GameHandler {
    event_tx: Sender<Event>,
}

impl GameHandler {
    /// Create a new game handler
    pub fn new(event_tx: Sender<Event>) -> Self {
        Self { event_tx }
    }

    /// Register game characteristics with the BLE service
    pub fn register_characteristics(
        &self,
        service: &Arc<NimbleMutex<BLEService>>,
        game_event_rx: Receiver<GameStateEvent>,
    ) -> Result<()> {
        // Action characteristic: phone -> board (writes)
        let action_characteristic = service.lock().create_characteristic(
            uuid128!(ACTION_CHARACTERISTIC_UUID),
            NimbleProperties::WRITE | NimbleProperties::WRITE_ENC,
        );

        // Event characteristic: board -> phone (notifications)
        let event_characteristic = service.lock().create_characteristic(
            uuid128!(EVENT_CHARACTERISTIC_UUID),
            NimbleProperties::READ | NimbleProperties::READ_ENC | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
        );

        // Setup action write handler
        {
            let event_tx = self.event_tx.clone();
            let buffer = Arc::new(Mutex::new(Vec::new()));

            action_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();
                let mut buffer = buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    match decode_action_frame(&frame) {
                        Ok(event) => {
                            if let Err(e) = event_tx.send(Event::GameCommand(event)) {
                                warn!("Failed to forward BLE game command event: {:?}", e);
                            }
                        }
                        Err(e) => warn!("Failed to decode incoming BLE action frame: {:?}", e),
                    }
                }
            });
        }

        // Start thread to forward game events to BLE
        thread::spawn(move || {
            info!("Game event sender thread started");
            while let Ok(event) = game_event_rx.recv() {
                let serializable = match event {
                    GameStateEvent::OngoingGamesLoaded(ongoing) => {
                        Some(SerializableGameStateEvent::OngoingGamesLoaded { games: ongoing })
                    }
                    GameStateEvent::GameLoaded(game_key) => {
                        Some(SerializableGameStateEvent::GameLoaded { game_key })
                    }
                    _ => None,
                };

                if let Some(evt) = serializable {
                    match encode_json_frame(&evt) {
                        Ok(frame) => {
                            send_chunked_notification(&event_characteristic, &frame);
                        }
                        Err(e) => {
                            warn!("Failed to encode game event: {:?}", e);
                        }
                    }
                }
            }
            info!("Game event sender thread exiting");
        });

        Ok(())
    }
}

fn decode_action_frame(payload: &[u8]) -> Result<GameCommandEvent> {
    serde_json::from_slice::<Frame<GameCommandEvent>>(payload)
        .map(|frame| frame.msg)
        .map_err(|e| BluetoothError::Protocol(e.to_string()))
}
