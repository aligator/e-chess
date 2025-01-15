use std::{fmt, result, str::FromStr};

use chess::{Color, Game};
use thiserror::Error;
use colored::*;

#[derive(Error, Debug)]
pub enum ChessGameError {
    #[error("board could not be loaded by the given FEN")]
    LoadingFen(#[from] chess::InvalidError),

    #[error("unknown chess game error")]
    Unknown,
}

pub struct ChessGame {
    pub game: Game,

    pub white_physical: u64, // Tracks physical pieces for white
    pub black_physical: u64, // Tracks physical pieces for black
}

impl fmt::Debug for ChessGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use chess::{Board, Square, Piece, Rank, File};
        
        let board: Board = self.game.current_position();
        
        // Add header showing whose turn it is
        let turn = if self.game.side_to_move() == Color::White {
            "White to move"
        } else {
            "Black to move"
        };
        writeln!(f, "{}", turn)?;
        
        // Add file labels at the top
        writeln!(f, "  a b c d e f g h")?;
        writeln!(f, "  ---------------")?;
        
        // Print board rows from top (rank 8) to bottom (rank 1)
        for rank in (0..8).rev() {
            write!(f, "{} ", rank + 1)?; // Rank number
            for file in 0..8 {
                let is_light_square = (rank + file) % 2 == 0;
                let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
                let piece = board.piece_on(square);
                let symbol = match piece {
                    Some(Piece::Pawn) => if board.color_on(square) == Some(Color::White) { "P" } else { "p" },
                    Some(Piece::Knight) => if board.color_on(square) == Some(Color::White) { "N" } else { "n" },
                    Some(Piece::Bishop) => if board.color_on(square) == Some(Color::White) { "B" } else { "b" },
                    Some(Piece::Rook) => if board.color_on(square) == Some(Color::White) { "R" } else { "r" },
                    Some(Piece::Queen) => if board.color_on(square) == Some(Color::White) { "Q" } else { "q" },
                    Some(Piece::King) => if board.color_on(square) == Some(Color::White) { "K" } else { "k" },
                    None => " ",
                };
                
                // Apply background color based on square
                let colored_symbol = if is_light_square {
                    format!(" {} ", symbol).on_truecolor(100, 100, 100)
                } else {
                    format!(" {} ", symbol).on_truecolor(180, 180, 180)
                };
                
                write!(f, "{}", colored_symbol)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  ---------------")?;
        writeln!(f, "  a b c d e f g h")
    }
}

impl ChessGame {
    pub fn new() -> Self {
        let initial_game = Game::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let white = initial_game.current_position().color_combined(Color::White).0;
        let black = initial_game.current_position().color_combined(Color::Black).0;

        ChessGame {
            game: initial_game,
            white_physical: white,
            black_physical: black,
        }
    }

    pub fn reset(&mut self, fen: &str) -> Result<(), ChessGameError> {
        self.game = Game::from_str(fen).map_err(ChessGameError::LoadingFen)?;

        // Reset expected physical board state based on the loaded game.
        self.white_physical = self.game.current_position().color_combined(Color::White).0;
        self.black_physical = self.game.current_position().color_combined(Color::Black).0;
        Ok(())        
    }

    pub fn physical(&self) -> u64 {
        return self.game().current_position().combined().0
    }

    /// Updates the game state based on the current board state
    /// The input bitboard represents the physical state of the board
    /// where 1 means a piece is present and 0 means empty
    pub fn tick(&mut self, physical_board: u64) -> u64 {
        // Update the game state based on the physical board
        let last_occupied = self.physical();
        let current_player = self.game.side_to_move();

        // If there is already a winner, just do nothing.
        if self.game.result().is_some() {
            return self.physical();
        }

        if last_occupied > physical_board {

        }

        /*
        let state = self.current();

        let last_occupied = state.get_occupied();
        let current_player = self.current_player();

        // If the new board is empty - reset the game.
        if now_occupied == 0 && self.current_index != 0 {
            info!("reset game");
            *self = TicTacToe::default()
        }

        // If there is already a winner, just do nothing.
        if state.winner.is_some() {
            return GameState {
                board: state,
                _player: current_player,
            };
        }

        // The new board must have more bits set - e.g. it must be a higher number.
        if last_occupied > now_occupied {
            return match self.last() {
                Some(last) => {
                    if last.get_occupied() != now_occupied {
                        // The new state is not the same like the last one.
                        // Do notheing
                        return GameState {
                            board: state,
                            _player: current_player,
                        };
                    }

                    // Undo one move.
                    let previous = self.pull();
                    return GameState {
                        board: previous,
                        _player: self.current_player(),
                    };
                }
                None => GameState {
                    board: state,
                    _player: current_player,
                },
            };
        } else if last_occupied == now_occupied {
            return GameState {
                board: state,
                _player: current_player,
            };
        }

        // First get all "different" fields.
        // Due to the check before, new bits can only come from the new_board.
        // Then only check if it is only 1 new bit. Else something must be wrong.
        let diff = only_different(now_occupied, last_occupied);
        let only_one = only_one_bit_set_to_one(diff);
        if !only_one {
            return GameState {
                board: state,
                _player: current_player,
            };
        }

        let mut new_state = state.clone();

        // Add the new field to the current player.
        new_state.players[current_player] = new_state.players[current_player] | diff;
        self.calculate_win(&mut new_state);
        self.push(new_state);

        return GameState {
            board: new_state,
            _player: self.current_player(),
        };
        
         */

        return self.physical();
    }

    pub fn game(&self) -> &Game {
        &self.game
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_game() {
        let chess = ChessGame::new();
        assert_eq!(chess.white_physical, 65535);

        // 11111111
        // 11111111
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        assert_eq!(chess.black_physical, 18446462598732840960);

        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 11111111
        // 11111111
        assert_eq!(chess.game().side_to_move(), Color::White);
    }
} 