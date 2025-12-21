use std::{
    fmt::Debug,
    sync::{mpsc::Sender, Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use chess::BitBoard;
use chess_game::{
    chess_connector::LocalChessConnector,
    game::{ChessGame, ChessGameError, ChessGameState},
};

use esp_idf_svc::nvs::NvsDefault;
use log::*;
use std::thread;
use std::thread::sleep;

use crate::{api, event::EventManager, storage::Storage, wifi::ConnectionStateEvent, Event};

#[derive(Clone)]
pub struct Settings {
    pub token: String,
    pub last_game_id: String,

    storage: Arc<Mutex<Storage<NvsDefault>>>,
}

impl Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings")
            .field("token", &self.token)
            .field("last_game_id", &self.last_game_id)
            .finish()
    }
}

impl Settings {
    pub fn new(storage: Storage<NvsDefault>) -> Result<Self> {
        Ok(Settings {
            token: storage.get_str::<25>("api_token")?.unwrap_or_default(),
            last_game_id: storage.get_str::<57>("last_game_id")?.unwrap_or_default(), // use 57 so it may be used for FEN strings also...

            storage: Arc::new(Mutex::new(storage)),
        })
    }

    pub fn save(&self) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();

        storage.set_str("api_token", &self.token)?;
        storage.set_str("last_game_id", &self.last_game_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// Events that are sent from the game thread to the main thread
pub enum GameStateEvent {
    UpdateGame(ChessGameState),
    GameLoaded(String),
}

#[derive(Debug, Clone)]
/// Events that are sent from the main thread to the game thread
pub enum GameCommandEvent {
    LoadNewGame(String),
    UpdatePhysical(BitBoard),
    RequestTakeBack,
    AcceptTakeBack,
}

fn load_game(
    game_key: String,
    settings: Arc<Mutex<Settings>>,
    tx: Sender<Event>,
) -> Result<ChessGame, ChessGameError> {
    // If the game key is a FEN string, parse it and start a local game.
    // Otherwise, start a lichess game.
    let mut chess_game = ChessGame::new(if game_key.contains(" ") {
        LocalChessConnector::new()
    } else {
        api::create(settings.clone()).unwrap()
    })?;

    // Load the new game
    match chess_game.reset(&game_key) {
        Ok(_) => {
            info!("Successfully reset game with ID: {}", game_key);
            // Notify the UI about the new game
            if let Some(state) = chess_game.get_state() {
                info!("Sending game update event: {:?}", state);
                if let Err(e) = tx.send(Event::GameState(GameStateEvent::UpdateGame(state))) {
                    warn!("Failed to send game update event: {:?}", e);
                }
            }

            // Send the GameLoaded event with the game ID
            if let Err(e) = tx.send(Event::GameState(GameStateEvent::GameLoaded(
                game_key.clone(),
            ))) {
                warn!("Failed to send game loaded event: {:?}", e);
            }

            // Update the last_game_id in settings
            let mut settings = settings.lock().unwrap();
            settings.last_game_id = game_key;
            settings.save().unwrap();

            Ok(chess_game)
        }
        Err(e) => {
            warn!("Failed to reset game: {:?}", e);

            // Send an empty GameLoaded event to indicate failure
            if let Err(e) = tx.send(Event::GameState(GameStateEvent::GameLoaded(String::new()))) {
                warn!("Failed to send game loaded event: {:?}", e);
            }
            Err(e)
        }
    }
}

pub fn run_game(event_manager: &EventManager<Event>, settings: Arc<Mutex<Settings>>) {
    let tx = event_manager.create_sender();
    let rx = event_manager.create_receiver();

    info!("Starting game thread");
    thread::spawn(move || {
        let mut chess_game: ChessGame = ChessGame::new(LocalChessConnector::new()).unwrap();
        info!("Created ChessGame");

        let mut physical = BitBoard::new(0);
        let mut last_game_state: Option<ChessGameState> = None;

        let mut wifi_connected = false;
        loop {
            // Sleep for 100ms to avoid busy-waiting
            sleep(Duration::from_millis(100));
            while let Ok(event) = rx.try_recv() {
                match event {
                    Event::ConnectionState(ConnectionStateEvent::Wifi(_wifi_info)) => {
                        wifi_connected = true;
                    }
                    Event::ConnectionState(ConnectionStateEvent::NotConnected) => {
                        wifi_connected = false;
                    }
                    // Handle WiFi connection state changes if needed
                    // Handle WiFi connection state changes if needed
                    Event::GameCommand(GameCommandEvent::UpdatePhysical(new_physical)) => {
                        physical = new_physical;
                    }
                    Event::GameCommand(GameCommandEvent::RequestTakeBack) => {
                        warn!("Not implemented");
                    }
                    Event::GameCommand(GameCommandEvent::AcceptTakeBack) => {
                        warn!("Not implemented");
                    }
                    Event::GameCommand(GameCommandEvent::LoadNewGame(game_id)) => {
                        if !game_id.contains(" ") && !wifi_connected {
                            warn!("Cannot load new game, WiFi not connected");
                            continue;
                        }

                        info!("Loading new game: {}", game_id);

                        match load_game(game_id, settings.clone(), tx.clone()) {
                            Ok(new_chess_game) => {
                                // Reset the game state so that it updates on the next tick
                                last_game_state = None;

                                // Replace the game instance.
                                chess_game = new_chess_game;
                            }
                            Err(e) => error!("Error loading game: {:?}", e),
                        }
                    }
                    _ => {}
                }
            }

            match chess_game.tick(physical) {
                Ok(()) => {
                    if let Some(state) = chess_game.get_state() {
                        if let Some(last_game_state_extracted) = last_game_state {
                            if last_game_state_extracted == state {
                                continue;
                            }
                        }

                        let event = Event::GameState(GameStateEvent::UpdateGame(state));
                        if let Err(e) = tx.send(event) {
                            error!("Failed to send new game state: {:?}", e);
                        }

                        last_game_state = Some(state)
                    } else {
                        //warn!("No game state found");
                    }
                }
                Err(e) => error!("Error ticking game: {:?}", e),
            }
        }
    });

    info!("Game thread started");
}
