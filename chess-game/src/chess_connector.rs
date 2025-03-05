use chess::ChessMove;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChessConnectorError {
    #[error("game not found")]
    GameNotFound,
}

pub trait ChessConnector {
    /// Loads a game by id and returns the FEN string of the game.
    fn load_game(&self, id: &str) -> Result<String, ChessConnectorError>;

    /// Make a move on the board.
    /// Else it will be executed and if it works return true, else false.
    /// If it is done by a player that is not a local player, it will be ignored and anyway return true.
    fn make_move(&self, chess_move: ChessMove) -> bool;
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
}

impl LocalChessConnector {
    pub fn new() -> Self {
        LocalChessConnector
    }
}
