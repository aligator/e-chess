use std::{sync::mpsc::Sender, time::Duration};

use chess::BitBoard;
use chess_game::{
    chess_connector::LocalChessConnector,
    game::{ChessGame, ChessGameError, ChessGameState},
    lichess::LichessConnector,
};
use log::*;
use std::thread;
use std::thread::sleep;

use crate::{event::EventManager, request::EspRequester, Event};

#[derive(Debug, Clone)]
pub struct Settings {
    pub token: String,
}

#[derive(Debug, Clone)]
/// Events that are sent from the game thread to the main thread
pub enum GameStateEvent {
    UpdateGame(BitBoard, ChessGameState),
    GameLoaded(String),
}

#[derive(Debug, Clone)]
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
    tx: Sender<Event>,
) -> Result<ChessGame, ChessGameError> {
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
            if let Some(state) = chess_game.get_state() {
                info!("Sending game update event: {:?}", state);
                if let Err(e) = tx.send(Event::GameState(GameStateEvent::UpdateGame(
                    state.physical,
                    state,
                ))) {
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

pub fn run_game(initial_settings: Settings, event_manager: &EventManager<Event>) {
    let tx = event_manager.create_sender();
    let rx = event_manager.create_receiver();

    info!("Starting game thread");
    thread::spawn(move || {
        let mut settings = initial_settings;

        let mut chess_game: ChessGame = ChessGame::new(LocalChessConnector::new()).unwrap();
        info!("Created ChessGame");

        let mut physical = BitBoard::new(0);
        let mut last_game_state: Option<ChessGameState> = None;
        loop {
            // Sleep for 100ms to avoid busy-waiting
            sleep(Duration::from_millis(100));
            while let Ok(event) = rx.try_recv() {
                match event {
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
                        info!("Loading new game: {}", game_id);

                        match load_game(game_id, &settings, tx.clone()) {
                            Ok(new_chess_game) => {
                                chess_game = new_chess_game;
                            }
                            Err(e) => error!("Error loading game: {:?}", e),
                        }
                    }
                    Event::GameCommand(GameCommandEvent::NewSettings(new_settings)) => {
                        settings = new_settings;
                    }
                    _ => {}
                }
            }

            match chess_game.tick(physical) {
                Ok(expected_physical) => {
                    if let Some(state) = chess_game.get_state() {
                        if let Some(last_game_state_extracted) = last_game_state {
                            if last_game_state_extracted == state {
                                continue;
                            }
                        }

                        let event =
                            Event::GameState(GameStateEvent::UpdateGame(expected_physical, state));
                        if let Err(e) = tx.send(event) {
                            error!("Failed to send new game state: {:?}", e);
                        }

                        last_game_state = Some(state)
                    } else {
                        warn!("No game state found");
                    }
                }
                Err(e) => error!("Error ticking game: {:?}", e),
            }
        }
    });

    info!("Game thread started");
}
