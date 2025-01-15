use std::{fmt, str::FromStr};

use crate::bitboard_extensions::*;
use chess::{BitBoard, Color, Game, Piece, Square};
#[cfg(feature = "colored")]
use colored::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChessGameError {
    #[error("board could not be loaded by the given FEN")]
    LoadingFen(#[from] chess::InvalidError),
}

pub struct ChessGame {
    pub game: Game,

    pub white_physical: u64, // Tracks physical pieces for white
    pub black_physical: u64, // Tracks physical pieces for black

    pub piece_moving_square: Option<Square>, // The square that the piece is moving from
    pub piece_moving: Option<Piece>,         // The piece that is currently moving
}

impl fmt::Debug for ChessGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use chess::{Board, File, Piece, Rank, Square};

        let board: Board = self.game.current_position();

        // Add header showing whose turn it is
        let turn = if self.game.side_to_move() == Color::White {
            "White to move"
        } else {
            "Black to move"
        };
        writeln!(f, "{}", turn)?;
        if self.piece_moving.is_some() {
            writeln!(
                f,
                "Moving piece: {:?} at {:?}",
                self.piece_moving, self.piece_moving_square
            )?;
        } else {
            writeln!(f, "No piece moving")?;
        }

        #[cfg(not(feature = "colored-debug"))]
        writeln!(f, "\n♙ = white\n♟ = black\n")?;

        // Add file labels at the top
        writeln!(f, "\n   a  b  c  d  e  f  g  h ")?;
        writeln!(f, "  ------------------------")?;

        // Print board rows from top (rank 8) to bottom (rank 1)
        for rank in (0..8).rev() {
            write!(f, "{} ", rank + 1)?; // Rank number
            for file in 0..8 {
                let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
                let piece = board.piece_on(square);

                let symbol = if board.color_on(square) == Some(Color::White) {
                    match piece {
                        Some(Piece::Pawn) => "♙",
                        Some(Piece::Knight) => "♘",
                        Some(Piece::Bishop) => "♗",
                        Some(Piece::Rook) => "♖",
                        Some(Piece::Queen) => "♕",
                        Some(Piece::King) => "♔",
                        None => " ",
                    }
                } else {
                    match piece {
                        Some(Piece::Pawn) => "♟",
                        Some(Piece::Knight) => "♞",
                        Some(Piece::Bishop) => "♝",
                        Some(Piece::Rook) => "♜",
                        Some(Piece::Queen) => "♛",
                        Some(Piece::King) => "♚",
                        None => " ",
                    }
                }
                .bold();

                #[cfg(feature = "colored-debug")]
                let symbol = if board.color_on(square) == Some(Color::White) {
                    symbol.truecolor(255, 255, 255)
                } else {
                    symbol.truecolor(0, 0, 0)
                };

                // Apply background color based on square and moving state
                #[cfg(feature = "colored-debug")]
                let colored_symbol = {
                    let is_light_square = (rank + file) % 2 == 0;
                    if Some(square) == self.piece_moving_square {
                        // Highlight moving square in green
                        format!(" {} ", symbol).on_green()
                    } else if is_light_square {
                        format!(" {} ", symbol).on_truecolor(110, 110, 110)
                    } else {
                        format!(" {} ", symbol).on_truecolor(130, 130, 130)
                    }
                };

                #[cfg(not(feature = "colored-debug"))]
                let colored_symbol = format!(" {} ", symbol);

                write!(f, "{}", colored_symbol)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  ------------------------")?;
        writeln!(f, "   a  b  c  d  e  f  g  h ")
    }
}

impl ChessGame {
    pub fn new() -> Self {
        let initial_game =
            Game::from_str("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let white = initial_game
            .current_position()
            .color_combined(Color::White)
            .0;
        let black = initial_game
            .current_position()
            .color_combined(Color::Black)
            .0;

        ChessGame {
            game: initial_game,
            white_physical: white,
            black_physical: black,

            piece_moving_square: None,
            piece_moving: None,
        }
    }

    pub fn reset(&mut self, fen: &str) -> Result<(), ChessGameError> {
        self.game = Game::from_str(fen).map_err(ChessGameError::LoadingFen)?;

        // Reset expected physical board state based on the loaded game.
        self.white_physical = self.game.current_position().color_combined(Color::White).0;
        self.black_physical = self.game.current_position().color_combined(Color::Black).0;
        Ok(())
    }

    pub fn physical(&self) -> BitBoard {
        self.game().current_position().combined().clone()
    }

    /// A new pice got placed.
    /// This move is only possible, if one pice was removed before (to make a move).
    fn place_physical(&mut self, square: Square) {}

    fn remove_physical(&mut self, square: Square) {
        // Remove piece, but remember it.

        // Check if a "move" is not already in progress.
        if self.piece_moving.is_some() {
            // Do nothing. It is illegal to remove a piece while a piece is already moving.
            return;
        }

        // Check if it is a piece of the current player.
        if self.game.current_position().color_on(square) != Some(self.game.side_to_move()) {
            // Do nothing. It is illegal to move pieces of the opponent.
            return;
        }

        // Remember the piece that is moving.
        self.piece_moving_square = Some(square);
        self.piece_moving = self.game.current_position().piece_on(square);

        // Remove the piece from the physical board.
        // Just do both at once - it is easier and still correct.
        self.white_physical ^= square.to_int() as u64;
        self.black_physical ^= square.to_int() as u64;
    }

    /// Updates the game state based on the current board state
    /// The input bitboard represents the physical state of the board
    /// where 1 means a piece is present and 0 means empty
    pub fn tick(&mut self, physical_board: BitBoard) -> BitBoard {
        // Update the game state based on the physical board
        let last_occupied = self.physical();

        // If there is already a winner, just do nothing.
        if self.game.result().is_some() {
            return last_occupied;
        }

        if last_occupied.only_one_bit_set_to_one() {
            // If more than one bit differs - do nothing,
            // as there would be no way to determine what happens.
            // In this case the previous physical board state has to be restored before continuing.

            // TODO: maybe later we may add a check here to handle "moving" a part physically.
            // That would be the case if the player moves the pice in a way that the reeds of both field change
            // their state at the same time.
            // However as I am not sure if that will be physically possible, I will leave it out for now.
            return last_occupied;
        }

        if physical_board.0 > last_occupied.0 {
            // If more bits are set than a piece must have been placed.
            self.place_physical(Square::new(
                last_occupied.get_different_bits(physical_board).first_one(),
            ));
            return BitBoard::new(self.white_physical | self.black_physical);
        } else if physical_board.0 < last_occupied.0 {
            // If less bits are set than a piece must have been removed.
            self.remove_physical(Square::new(
                last_occupied.get_different_bits(physical_board).first_one(),
            ));
            return BitBoard::new(self.white_physical | self.black_physical);
        } else {
            // If the same number of bits are set, nothing has changed.
            return last_occupied;
        }
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
