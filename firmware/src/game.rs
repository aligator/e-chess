use std::sync::mpsc;

use chess::BitBoard;
use chess_game::{
    chess_connector::LocalChessConnector,
    game::{ChessGame, ChessGameError},
    lichess::LichessConnector,
};
use log::*;

use crate::request::EspRequester;

#[derive(Debug, Clone)]
pub struct Settings {
    pub token: String,
}

#[derive(Debug)]
/// Events that are sent from the game thread to the main thread
pub enum GameStateEvent {
    UpdateGame(Option<chess::Game>),
    ExpectedPhysical(BitBoard),
    GameLoaded(String),
}

#[derive(Debug)]
/// Events that are sent from the main thread to the game thread
pub enum GameCommandEvent {
    LoadNewGame(String),
    UpdatePhysical(BitBoard),
    RequestTakeBack,
    AcceptTakeBack,

    NewSettings(Settings),
}

fn load_game(
    game_key: String,
    settings: &Settings,
    state_tx: mpsc::Sender<GameStateEvent>,
) -> Result<ChessGame, ChessGameError> {
    info!("Loading new game: {}", game_key);

    // If the game key is a FEN string, parse it and start a local game.
    // Otherwise, start a lichess game.
    let mut chess_game = ChessGame::new(if game_key.contains(" ") {
        LocalChessConnector::new()
    } else {
        let requester = EspRequester::new(settings.token.clone());
        Box::new(LichessConnector::new(requester))
    })?;

    // Load the new game
    match chess_game.reset(&game_key) {
        Ok(_) => {
            info!("Successfully reset game with ID: {}", game_key);
            // Notify the UI about the new game
            match state_tx.send(GameStateEvent::UpdateGame(chess_game.game())) {
                Ok(_) => info!("Sent game update event (new game)"),
                Err(e) => warn!("Failed to send game update event: {:?}", e),
            }

            // Send the GameLoaded event with the game ID
            match state_tx.send(GameStateEvent::GameLoaded(game_key.clone())) {
                Ok(_) => info!("Sent game loaded event for ID: {}", game_key),
                Err(e) => warn!("Failed to send game loaded event: {:?}", e),
            }
            Ok(chess_game)
        }
        Err(e) => {
            warn!("Failed to reset game: {:?}", e);

            // Send an empty GameLoaded event to indicate failure
            match state_tx.send(GameStateEvent::GameLoaded(String::new())) {
                Ok(_) => info!("Sent empty game loaded event to indicate failure"),
                Err(e) => warn!("Failed to send game loaded event: {:?}", e),
            }
            Err(e)
        }
    }
}

pub fn run_game(
    initial_settings: Settings,
    rx: mpsc::Receiver<GameCommandEvent>,
    tx: mpsc::Sender<GameStateEvent>,
) {
    let _game_thread = std::thread::Builder::new()
        .spawn(move || {
            let mut settings = initial_settings;

            let mut chess_game: ChessGame = ChessGame::new(LocalChessConnector::new()).unwrap();
            info!("Created ChessGame");
            chess_game.reset("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

            let mut physical = BitBoard::new(0);
            loop {
                match rx.try_recv() {
                    Ok(event) => match event {
                        GameCommandEvent::UpdatePhysical(new_physical) => {
                            physical = new_physical;
                        }
                        GameCommandEvent::RequestTakeBack => {
                            warn!("Not implemented");
                        }
                        GameCommandEvent::AcceptTakeBack => {
                            warn!("Not implemented");
                        }
                        GameCommandEvent::LoadNewGame(game_id) => {
                            match load_game(game_id, &settings, tx.clone()) {
                                Ok(new_chess_game) => {
                                    chess_game = new_chess_game;
                                }
                                Err(e) => error!("Error loading game: {:?}", e),
                            }
                        }
                        GameCommandEvent::NewSettings(new_settings) => {
                            settings = new_settings;
                        }
                    },
                    Err(e) => error!("Error receiving event: {:?}", e),
                }

                match chess_game.tick(physical) {
                    Ok(new_physical) => {
                        tx.send(GameStateEvent::ExpectedPhysical(new_physical))
                            .unwrap();
                    }
                    Err(e) => error!("Error ticking game: {:?}", e),
                };
            }
        })
        .unwrap();
}
