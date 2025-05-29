use std::{str::FromStr, sync::mpsc::Sender};

use chess::{ChessMove, Game};
use thiserror::Error;

use crate::event::GameEvent;

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

pub trait ChessConnector {
    /// Loads a game by id and returns the Game.
    fn load_game(&mut self, id: &str, tx: Sender<GameEvent>) -> Result<Game, ChessConnectorError>;

    /// Make a move on the board.
    /// Else it will be executed and if it works return true, else false.
    /// If it is done by a player that is not a local player, it will be ignored and anyway return true.
    fn make_move(&self, chess_move: ChessMove) -> bool;
}

pub struct LocalChessConnector;

impl ChessConnector for LocalChessConnector {
    /// Loads a game by initializing a new game with the starting position.
    fn load_game(&mut self, id: &str, _tx: Sender<GameEvent>) -> Result<Game, ChessConnectorError> {
        Ok(Game::from_str(id).map_err(|_| ChessConnectorError::InvalidFen(id.to_string()))?)
    }

    fn make_move(&self, _chess_move: ChessMove) -> bool {
        true
    }
}

impl LocalChessConnector {
    pub fn new() -> Box<dyn ChessConnector> {
        Box::new(LocalChessConnector)
    }
}
