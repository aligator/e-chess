use std::str::FromStr;

use chess::{ChessMove, Game};
use thiserror::Error;

use crate::requester::{DummyRequester, Requester};

#[derive(Error, Debug)]
pub enum ChessConnectorError<R: Requester> {
    #[error("game not found")]
    GameNotFound,
    #[error("request error")]
    RequestError(#[source] R::RequestError),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("invalid fen: {0}")]
    InvalidFen(String),
}

pub trait ChessConnector {
    type R: Requester;

    /// Loads a game by id and returns the FEN string of the game.
    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError<Self::R>>;

    /// Make a move on the board.
    /// Else it will be executed and if it works return true, else false.
    /// If it is done by a player that is not a local player, it will be ignored and anyway return true.
    fn make_move(&self, chess_move: ChessMove) -> bool;

    /// Ticks the connector and updates the board by returning the FEN string of the game.
    /// In this function the connector can check for new upstream events.
    /// It gets called as often as possible, so it should be lightweight.
    fn tick(&self) -> Result<Option<String>, ChessConnectorError<Self::R>>;
}

pub struct LocalChessConnector;

impl ChessConnector for LocalChessConnector {
    type R = DummyRequester;

    /// Loads a game by initializing a new game with the starting position.
    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError<Self::R>> {
        if id == "" {
            Game::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        } else {
            Game::from_str(id)
        }
        .map_err(|_| ChessConnectorError::<Self::R>::InvalidFen(id.to_string()))
    }

    fn make_move(&self, _chess_move: ChessMove) -> bool {
        true
    }

    fn tick(&self) -> Result<Option<String>, ChessConnectorError<Self::R>> {
        Ok(Some(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
        ))
    }
}

impl LocalChessConnector {
    pub fn new() -> Self {
        LocalChessConnector
    }
}
