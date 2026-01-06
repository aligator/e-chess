use crate::util::bitboard_serializer;
use crate::{bluetooth::BluetoothService, event::EventManager, Event};
use anyhow::Result;
use chess::BitBoard;
use chess_game::chess_connector::OngoingGame;
use chess_game::{
    chess_connector::{ChessConnector, LocalChessConnector},
    game::{ChessGame, ChessGameError, ChessGameState},
    lichess::LichessConnector,
};
use log::*;
use serde::{Deserialize, Serialize};
use std::thread;
use std::thread::sleep;
use std::{
    fmt::Debug,
    sync::{mpsc::Sender, Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Clone)]
/// Events that are sent from the game thread to the main thread
pub enum GameStateEvent {
    OngoingGamesLoaded(Vec<OngoingGame>),
    UpdateGame(ChessGameState),
    GameLoaded(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameCommandEvent {
    LoadOpenGames,
    LoadNewGame {
        game_key: String,
    },
    UpdatePhysical {
        #[serde(with = "bitboard_serializer")]
        bitboard: BitBoard,
    },
    RequestTakeBack,
    AcceptTakeBack,
}

fn load_game(
    game_key: String,
    tx: Sender<Event>,
    connectors: &[Arc<Mutex<dyn ChessConnector + Send>>],
) -> Result<ChessGame, ChessGameError> {
    // If the game key is a FEN string, parse it and start a local game.
    // Otherwise, start a lichess game.
    let mut chess_game: Option<ChessGame> = None;
    for connector in connectors {
        if connector.lock().unwrap().is_valid_key(game_key.clone()) {
            chess_game = Some(ChessGame::new(connector.clone())?);
            break;
        }
    }

    let mut chess_game = if chess_game.is_none() {
        return Err(ChessGameError::InvalidGameKey);
    } else {
        chess_game.unwrap()
    };

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

pub fn run_game(event_manager: &EventManager<Event>) {
    let event_tx = event_manager.create_sender();
    let event_rx = event_manager.create_receiver();

    let (_ble_service, bridge_handler, game_event_tx) =
        BluetoothService::new("E-Chess", Duration::from_secs(10), event_tx.clone())
            .expect("Failed to initialize Bluetooth service");

    let connectors: Vec<Arc<Mutex<dyn ChessConnector + Send>>> = vec![
        Arc::new(Mutex::new(LocalChessConnector {})),
        Arc::new(Mutex::new(LichessConnector::new(bridge_handler))),
    ];

    info!("Starting game thread");
    thread::spawn(move || {
        let mut chess_game: ChessGame = ChessGame::new(connectors[0].clone()).unwrap();
        info!("Created ChessGame");

        let mut physical = BitBoard::new(0);
        let mut last_game_state: Option<ChessGameState> = None;

        loop {
            // Sleep for 100ms to avoid busy-waiting
            sleep(Duration::from_millis(100));
            while let Ok(event) = event_rx.try_recv() {
                // Send game state events to BLE
                if let Event::GameState(ref game_state) = event {
                    let _ = game_event_tx.send(game_state.clone());
                }

                match event {
                    Event::GameCommand(GameCommandEvent::UpdatePhysical { bitboard }) => {
                        physical = bitboard;
                    }
                    Event::GameCommand(GameCommandEvent::RequestTakeBack) => {
                        warn!("Not implemented");
                    }
                    Event::GameCommand(GameCommandEvent::AcceptTakeBack) => {
                        warn!("Not implemented");
                    }
                    Event::GameCommand(GameCommandEvent::LoadOpenGames) => {
                        info!("Loading open games");
                        let mut open_games: Vec<OngoingGame> = vec![];
                        for connector in &connectors {
                            match connector.lock().unwrap().find_open_games() {
                                Ok(games) => {
                                    open_games.extend(games.clone());
                                }
                                Err(e) => error!("Error loading open games: {:?}", e),
                            }
                        }
                        event_tx
                            .send(Event::GameState(GameStateEvent::OngoingGamesLoaded(
                                open_games,
                            )))
                            .map_err(|err| error!("Error sending ongoing games: {:?}", err))
                            .ok();
                    }
                    Event::GameCommand(GameCommandEvent::LoadNewGame { game_key }) => {
                        info!("Loading new game: {}", game_key);
                        match load_game(game_key, event_tx.clone(), &connectors) {
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
                        if let Err(e) = event_tx.send(event) {
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
