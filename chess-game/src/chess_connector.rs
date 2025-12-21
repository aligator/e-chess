use std::str::FromStr;

use chess::{ChessMove, Game};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChessConnectorError {
    #[error("game not found")]
    GameNotFound,
    #[error("request error: {0}")]
    RequestError(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("invalid fen: {0}")]
    InvalidFen(String),
}

pub struct GameState {
    pub white_request_take_back: bool,
    pub black_request_take_back: bool,
    pub moves: Vec<String>,
}

pub enum GameEvent {
    State(GameState),
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerInfo {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OngoingGame {
    pub game_id: String,
    pub opponent: PlayerInfo,
}

pub trait ChessConnector {
    /// Find open games.
    fn find_open_games(&self) -> Result<Vec<OngoingGame>, ChessConnectorError>;

    /// Loads a game by id and returns the Game.
    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError>;

    /// Make a move on the board.
    /// Else it will be executed and if it works return true, else false.
    /// If it is done by a player that is not a local player, it will be ignored and anyway return true.
    fn make_move(&self, chess_move: ChessMove) -> bool;

    /// Ticks the connector and updates the board by returning the FEN string of the game.
    /// In this function the connector can check for new upstream events.
    /// It gets called as often as possible, so it should be lightweight.
    fn next_event(&self) -> Result<Option<GameEvent>, ChessConnectorError>;
}

pub struct LocalChessConnector;

impl ChessConnector for LocalChessConnector {
    fn find_open_games(&self) -> Result<Vec<OngoingGame>, ChessConnectorError> {
        Ok(vec![OngoingGame {
            game_id: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            opponent: PlayerInfo {
                id: "local_opponent".to_string(),
                username: "Local Opponent".to_string(),
            },
        }])
    }

    /// Loads a game by initializing a new game with the starting position.
    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError> {
        Game::from_str(id).map_err(|_| ChessConnectorError::InvalidFen(id.to_string()))
    }

    fn make_move(&self, _chess_move: ChessMove) -> bool {
        true
    }

    fn next_event(&self) -> Result<Option<GameEvent>, ChessConnectorError> {
        Ok(None)
    }
}

impl LocalChessConnector {
    pub fn new() -> Box<dyn ChessConnector> {
        Box::new(LocalChessConnector)
    }
}
