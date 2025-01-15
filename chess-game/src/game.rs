use crate::bitboard_extensions::*;
use chess::{BitBoard, Board, ChessMove, Color, File, Game, MoveGen, Piece, Rank, Square};
#[cfg(feature = "colored")]
use colored::*;
use std::fmt::{Error, Write};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChessGameError {
    #[error("board could not be loaded by the given FEN")]
    LoadingFen(#[from] chess::InvalidError),
}

#[derive(Clone, Copy)]
pub enum ChessState {
    Idle,
    MovingPiece { piece: Piece, from: Square },
}

pub struct ChessGame {
    pub game: Game,

    pub white_physical: BitBoard, // Tracks physical pieces for white
    pub black_physical: BitBoard, // Tracks physical pieces for black

    pub state: ChessState,
}

impl fmt::Debug for ChessGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let board: Board = self.game.current_position();
        // Add header showing whose turn it is
        let turn = if self.game.side_to_move() == Color::White {
            "White to move"
        } else {
            "Black to move"
        };
        writeln!(f, "{}", turn)?;

        match self.state {
            ChessState::MovingPiece { piece, from } => {
                writeln!(f, "Moving piece: {:?} at {:?}", piece, from)?;
            }
            ChessState::Idle => {
                writeln!(f, "No action in progress")?;
            }
        }

        writeln!(f, "\nFEN: {}\n", self.game.current_position().to_string())?;

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
                };

                #[cfg(feature = "colored-debug")]
                let symbol = if board.color_on(square) == Some(Color::White) {
                    symbol.bold().truecolor(255, 255, 255)
                } else {
                    symbol.bold().truecolor(0, 0, 0)
                };

                // Apply background color based on square and moving state
                #[cfg(feature = "colored-debug")]
                let colored_symbol = {
                    let colored_symbol = {
                        let is_light_square = (rank + file) % 2 == 0;
                        if is_light_square {
                            format!(" {} ", symbol).on_truecolor(110, 110, 110)
                        } else {
                            format!(" {} ", symbol).on_truecolor(130, 130, 130)
                        }
                    };

                    // Colorize the moving piece.
                    let colored_symbol =
                        if let ChessState::MovingPiece { piece: _, from } = self.state {
                            if square == from {
                                // Highlight moving square in green
                                format!(" {} ", symbol).on_green()
                            } else {
                                // TODO: is it performant to call this multiple times?
                                if MoveGen::new_legal(&self.game.current_position())
                                    .filter(|m| m.get_source() == from)
                                    .any(|m| m.get_dest() == square)
                                {
                                    format!(" {} ", symbol).on_blue()
                                } else {
                                    colored_symbol
                                }
                            }
                        } else {
                            colored_symbol
                        };

                    colored_symbol
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
            .clone();
        let black = initial_game
            .current_position()
            .color_combined(Color::Black)
            .clone();

        ChessGame {
            game: initial_game,
            white_physical: white,
            black_physical: black,
            state: ChessState::Idle,
        }
    }

    pub fn reset(&mut self, fen: &str) -> Result<(), ChessGameError> {
        self.game = Game::from_str(fen).map_err(ChessGameError::LoadingFen)?;

        // Reset expected physical board state based on the loaded game.
        self.white_physical = self
            .game
            .current_position()
            .color_combined(Color::White)
            .clone();
        self.black_physical = self
            .game
            .current_position()
            .color_combined(Color::Black)
            .clone();
        Ok(())
    }

    pub fn physical(&self) -> BitBoard {
        return self.white_physical | self.black_physical;
    }

    /// A new pice got placed.
    /// This move is only possible, if one pice was removed before (to make a move).
    fn place_physical(&mut self, to: Square) {
        match self.state {
            ChessState::MovingPiece { piece: _, from } => {
                // Allow just replacing it on the same square.
                if from != to {
                    let chess_move = ChessMove::new(from, to, None);

                    // Execute move. If it is illegal do not proceed.
                    // TODO: test this on the micro controller. It may be slow!
                    if !self.game.make_move(chess_move) {
                        // Do nothing. It is illegal to place a piece on an illegal square.
                        return;
                    }
                }

                // Update the state with the moving piece
                self.state = ChessState::Idle;

                // Update the expected physical board states.
                // This includes any remove or castled pieces.
                self.white_physical = self
                    .game
                    .current_position()
                    .color_combined(Color::White)
                    .clone();
                self.black_physical = self
                    .game
                    .current_position()
                    .color_combined(Color::Black)
                    .clone();
            }
            ChessState::Idle => {
                // Do nothing. It is illegal to place a piece without removing one first.
            }
        }
    }

    // Remove piece physically, but remember it, so that it can be placed again later at another position.
    fn remove_physical(&mut self, square: Square) {
        match self.state {
            ChessState::MovingPiece { piece: _, from: _ } => {
                // Do nothing. It is illegal to remove a piece while a piece is already moving.
                return;
            }
            ChessState::Idle => {
                // Check if it is a piece of the current player.
                if self.game.current_position().color_on(square) != Some(self.game.side_to_move()) {
                    // Do nothing. It is illegal to move pieces of the opponent.
                    return;
                }

                // Update the state with the moving piece
                if let Some(piece) = self.game.current_position().piece_on(square) {
                    self.state = ChessState::MovingPiece {
                        piece,
                        from: square,
                    };
                }

                // Remove the piece from the physical board.
                // Just do both at once - it is easier and still correct.
                let bit = BitBoard::from_square(square);

                if self.game.side_to_move() == Color::White {
                    self.white_physical ^= bit;
                } else {
                    self.black_physical ^= bit;
                }
            }
        }
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
            return self.physical();
        } else if physical_board.0 < last_occupied.0 {
            // If less bits are set than a piece must have been removed.
            self.remove_physical(Square::new(
                last_occupied.get_different_bits(physical_board).first_one(),
            ));
            return self.physical();
        } else {
            // If the same number of bits are set,
            //
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
        assert_eq!(chess.white_physical, BitBoard::new(65535));

        // 11111111
        // 11111111
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        // 00000000
        assert_eq!(chess.black_physical, BitBoard::new(18446462598732840960));

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
