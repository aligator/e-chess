use chess::ChessMove;
use thiserror::Error;

use crate::requester::RequestError;

#[derive(Error, Debug)]
pub enum ChessConnectorError {
    #[error("game not found")]
    GameNotFound,
    #[error("request error")]
    RequestError(#[from] RequestError),
}

pub trait ChessConnector {
    /// Loads a game by id and returns the FEN string of the game.
    fn load_game(&self, id: &str) -> Result<String, ChessConnectorError>;

    /// Make a move on the board.
    /// Else it will be executed and if it works return true, else false.
    /// If it is done by a player that is not a local player, it will be ignored and anyway return true.
    fn make_move(&self, chess_move: ChessMove) -> bool;

    /// Ticks the connector and updates the board by returning the FEN string of the game.
    /// In this function the connector can check for new upstream events.
    /// It gets called as often as possible, so it should be lightweight.
    fn tick(&self) -> Result<String, ChessConnectorError>;
}

pub struct LocalChessConnector;

impl ChessConnector for LocalChessConnector {
    /// Loads a game by initializing a new game with the starting position.
    fn load_game(&self, id: &str) -> Result<String, ChessConnectorError> {
        if id == "" {
            Ok("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string())
        } else {
            Ok(id.to_string())
        }
    }

    fn make_move(&self, _chess_move: ChessMove) -> bool {
        true
    }

    fn tick(&self) -> Result<String, ChessConnectorError> {
        Ok("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string())
    }
}

impl LocalChessConnector {
    pub fn new() -> Self {
        LocalChessConnector
    }
}
