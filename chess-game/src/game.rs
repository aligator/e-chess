use crate::bitboard_extensions::*;
use crate::chess_connector::{ChessConnector, ChessConnectorError, GameEvent};
use chess::{Action, BitBoard, Board, ChessMove, Color, File, Game, MoveGen, Piece, Rank, Square};

#[cfg(feature = "colored")]
use colored::*;
use std::cmp::Ordering::*;
use std::fmt::Debug;
use std::{fmt, str::FromStr};
use thiserror::Error;

fn action_to_move(action: &Action) -> ChessMove {
    if let Action::MakeMove(m) = action {
        *m
    } else {
        panic!("Last move is not a make move");
    }
}

fn is_move_action(action: &&Action) -> bool {
    matches!(action, Action::MakeMove(_))
}

#[derive(Error, Debug)]
pub enum ChessGameError {
    #[error("board could not be loaded by the given FEN")]
    LoadingFen(#[from] chess::InvalidError),

    #[error("game could not be loaded")]
    LoadingGame(#[from] ChessConnectorError),
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlayingState {
    Idle,
    MovingPiece { piece: Piece, from: Square },
}

#[derive(Clone, Copy, PartialEq)]
pub struct ChessGameState {
    pub physical: BitBoard,
    pub expected_physical: BitBoard,
    pub playing_state: PlayingState,
    pub last_move: Option<ChessMove>,
    pub possible_moves: BitBoard,
    pub current_position: Board,
    pub active_player: Color,
}

impl Debug for ChessGameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ChessGameState {{ physical: {:?}, expected_physical: {:?}, last_move: {:?}, possible_moves: {:?} }}",
            self.physical, self.expected_physical, self.last_move, self.possible_moves
        )
    }
}

pub struct ChessGame {
    /// The local game representation.
    /// It uses the chess lib that implements the rules of chess.
    /// This makes it
    /// 1. possible to run a fully local chess game
    /// 2. possible to validate moves before they are sent to the server
    game: Option<Game>,

    /// The connection to the server.
    /// It is used to sync the game state with the server.
    /// It also provides events the local game listens to.
    /// For example if the opponent made a move, the local game will be notified.
    connection: Box<dyn ChessConnector>,

    /// The expected physical board state for white.
    expected_white: BitBoard,

    /// The expected physical board state for black.
    expected_black: BitBoard,

    /// The physical board state.
    /// If it differs too much from the expected state, the game pauses until it matches again.
    physical: BitBoard,

    /// The current physical state of the game.
    /// It indicates if a pice is currently being moved physically.
    playing_state: PlayingState,

    /// The last move that was made online.
    /// This is used to avoid sending the same moves multiple times.
    /// And to track if the amount of moves is less - which triggers a take back
    server_moves: Vec<ChessMove>,

    /// Current game id.
    /// Needed to reset the game in case of "undo" since the chess lib does not support undoing.
    id: String,
}

impl fmt::Debug for ChessGame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.game.is_none() {
            return write!(f, "No game");
        }

        let game = self.game.as_ref().unwrap();

        let board: Board = game.current_position();
        // Add header showing whose turn it is
        let turn = if game.side_to_move() == Color::White {
            "White to move"
        } else {
            "Black to move"
        };
        writeln!(f, "{}", turn)?;

        match self.playing_state {
            PlayingState::MovingPiece { piece, from } => {
                writeln!(f, "Moving piece: {:?} at {:?}", piece, from)?;
            }
            PlayingState::Idle => {
                writeln!(f, "No action in progress")?;
            }
        }

        writeln!(f, "\nFEN: {}\n", game.current_position())?;

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

                    // Colorize the last moved piece.
                    let last_move = self.last_move();
                    let colored_symbol = if let Some(last_move) = last_move {
                        if square == last_move.get_source() || square == last_move.get_dest() {
                            format!(" {} ", symbol).on_truecolor(0, 110, 110)
                        } else {
                            colored_symbol
                        }
                    } else {
                        colored_symbol
                    };

                    // Colorize the moving piece.
                    let colored_symbol =
                        if let PlayingState::MovingPiece { piece: _, from } = self.playing_state {
                            if square == from {
                                // Highlight moving square in green
                                format!(" {} ", symbol).on_green()
                            } else {
                                // TODO: is it performant enough to call this multiple times?
                                if MoveGen::new_legal(&game.current_position())
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

                    // Colorize pieces that should be set but aren't set with blue.
                    let colored_symbol = if (self.expected_white | self.expected_black).get(square)
                        == 1
                        && self.physical.get(square) == 0
                    {
                        format!(" {} ", symbol).on_blue()
                    } else {
                        colored_symbol
                    };

                    // Colorize pieces that are set but shouldn't be with red.
                    if self.physical.get(square) == 1
                        && (self.expected_white | self.expected_black).get(square) == 0
                    {
                        format!(" {} ", symbol).on_red()
                    } else {
                        colored_symbol
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
    pub fn new(connection: Box<dyn ChessConnector>) -> Result<Self, ChessGameError> {
        Ok(ChessGame {
            game: None,
            connection,
            expected_white: BitBoard(0),
            expected_black: BitBoard(0),
            physical: BitBoard::new(0),
            playing_state: PlayingState::Idle,
            server_moves: Vec::new(),
            id: String::new(),
        })
    }

    pub fn game_id(&self) -> String {
        self.id.clone()
    }

    pub fn game(&self) -> Option<Game> {
        self.game.clone()
    }

    pub fn last_move(&self) -> Option<ChessMove> {
        if let Some(game) = &self.game {
            game.actions()
                .iter()
                .filter(is_move_action)
                .last()
                .map(action_to_move)
        } else {
            None
        }
    }

    pub fn reset(&mut self, id: &str) -> Result<(), ChessGameError> {
        self.game = Some(self.connection.load_game(id)?);
        self.id = id.to_string();

        if let Some(game) = &self.game {
            self.server_moves = game
                .actions()
                .iter()
                .filter(is_move_action)
                .map(action_to_move)
                .collect();

            // Reset expected physical board state based on the loaded game.
            self.expected_white = *game.current_position().color_combined(Color::White);
            self.expected_black = *game.current_position().color_combined(Color::Black);
        }

        Ok(())
    }

    pub fn expected_physical(&self) -> BitBoard {
        self.expected_white | self.expected_black
    }

    fn execute_move(&mut self, chess_move: ChessMove) -> bool {
        if self.game.is_none() {
            return false;
        }

        let game = self.game.as_mut().unwrap();

        // First check if the move is legal.
        if !game.current_position().legal(chess_move) {
            return false;
        }

        // Check if the move has already been made.
        if self.server_moves.last() != Some(&chess_move) {
            // Ensure the move is legal by checking the connection first
            if !self.connection.make_move(chess_move) {
                return false;
            }
            self.server_moves.push(chess_move);
        }

        // If it was successful, execute the move also locally
        // -> should not fail as it is legal.
        if !game.make_move(chess_move) {
            panic!(
                "Move was legal but could not be executed locally. Should not happen. {:?}",
                chess_move
            );
        }
        true
    }

    /// A new pice got placed.
    /// This move is only possible, if one pice was removed before (to make a move).
    fn place_physical(&mut self, to: Square) {
        if self.game.is_none() {
            return;
        }

        match self.playing_state {
            PlayingState::MovingPiece { piece, from } => {
                // Only set promotion if it's a pawn moving to the last rank
                let promotion = if piece == Piece::Pawn {
                    let rank_idx = to.get_rank().to_index();
                    // For white pawns, promotion happens on rank 8 (index 7)
                    // For black pawns, promotion happens on rank 1 (index 0)
                    if rank_idx == 0 || rank_idx == 7 {
                        // TODO: make promotion piece somehow configurable.
                        Some(Piece::Queen)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let chess_move = ChessMove::new(from, to, promotion);

                // Allow just replacing it on the same square.
                if from != to {
                    // First check if the move is legal.
                    if !self.execute_move(chess_move) {
                        return;
                    }
                }

                // Update the state with the moving piece
                self.playing_state = PlayingState::Idle;

                // Update the expected physical board states.
                // This includes any remove or castled pieces.
                let game: &Game = self.game.as_ref().unwrap();
                self.expected_white = *game.current_position().color_combined(Color::White);
                self.expected_black = *game.current_position().color_combined(Color::Black);
            }
            PlayingState::Idle => {
                // Illegal to place piece without removing one first
            }
        }
    }

    // Remove piece physically, but remember it, so that it can be placed again later at another position.
    fn remove_physical(&mut self, square: Square) {
        if self.game.is_none() {
            return;
        }

        match self.playing_state {
            PlayingState::MovingPiece { piece: _, from } => {
                // This is only allowed if a piece is removed because it gets destroyed.
                // So if it is enemy and target of an attack by te moving piece.

                // Check if the piece is an enemy.
                {
                    let game: &Game = self.game.as_ref().unwrap();
                    if game.current_position().color_on(square) == Some(game.side_to_move()) {
                        // Do nothing. It is illegal to remove a piece of the current player.
                        return;
                    }
                }

                // Execute the move if it is successful - it is legal. If not, just do nothing.
                let chess_move = ChessMove::new(from, square, None);
                if !self.execute_move(chess_move) {
                    return;
                }

                // Update the state with the moving piece
                self.playing_state = PlayingState::Idle;

                // Update the expected physical board states.
                // This includes any remove pieces.
                // The player will have to place the pice on the enemies square to continue the game.
                let game: &Game = self.game.as_ref().unwrap();
                self.expected_white = *game.current_position().color_combined(Color::White);
                self.expected_black = *game.current_position().color_combined(Color::Black);
            }
            PlayingState::Idle => {
                let game: &Game = self.game.as_ref().unwrap();
                // Check if it is a piece of the current player.
                if game.current_position().color_on(square) != Some(game.side_to_move()) {
                    // Do nothing. It is illegal to move pieces of the opponent.
                    return;
                }

                // Update the state with the moving piece
                if let Some(piece) = game.current_position().piece_on(square) {
                    self.playing_state = PlayingState::MovingPiece {
                        piece,
                        from: square,
                    };
                }

                // Remove the piece from the physical board.
                // Just do both at once - it is easier and still correct.
                let bit = BitBoard::from_square(square);
                if game.side_to_move() == Color::White {
                    self.expected_white ^= bit;
                } else {
                    self.expected_black ^= bit;
                }
            }
        }
    }

    pub fn get_possible_moves(&self) -> BitBoard {
        if self.game.is_none() {
            return BitBoard::new(0);
        }

        let mut moves = BitBoard::new(0);

        if let PlayingState::MovingPiece { piece: _, from } = self.playing_state {
            let game: &Game = self.game.as_ref().unwrap();
            for m in MoveGen::new_legal(&game.current_position()).filter(|m| m.get_source() == from)
            {
                moves |= BitBoard::from_square(m.get_dest());
            }
        };

        moves
    }

    /// Updates the game state based on the current board state
    /// The input bitboard represents the physical state of the board
    /// where 1 means a piece is present and 0 means empty
    pub fn tick(&mut self, physical_board: BitBoard) -> Result<(), ChessGameError> {
        if self.game.is_none() {
            return Ok(());
        }
        let mut white_request_take_back = false;
        let mut black_request_take_back = false;
        let mut reset = false;

        {
            let game: &mut Game = self.game.as_mut().unwrap();

            // Tick the connection to get events until there is no more event.
            while let Some(event) = self.connection.next_event()? {
                match event {
                    GameEvent::State(state) => {
                        // Handle take-back.
                        white_request_take_back = state.white_request_take_back;
                        black_request_take_back = state.black_request_take_back;
                        // If the new moves are less than before - it is a take back.
                        if state.moves.len() <= self.server_moves.len() {
                            // Do it after the while to avoid problems with multiple mut refs of self.
                            reset = true;
                            break;
                        };

                        let last_move = state.moves.last();

                        if let Some(last_move) = last_move {
                            // event is the last move
                            match ChessMove::from_str(last_move) {
                                Ok(chess_move) => {
                                    game.make_move(chess_move);
                                    self.server_moves.push(chess_move);

                                    self.expected_white =
                                        *game.current_position().color_combined(Color::White);
                                    self.expected_black =
                                        *game.current_position().color_combined(Color::Black);
                                }
                                Err(e) => {
                                    return Err(ChessGameError::LoadingFen(e));
                                }
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        if reset {
            self.reset(self.id.clone().as_str())?;
            // And do the tick again to avoid missing events
            return self.tick(physical_board);
        }

        if white_request_take_back || black_request_take_back {
            println!("WARNING: do something with request_take_back");
        }

        // Save current physical board for visualization.
        self.physical = physical_board;

        // Update the game state based on the physical board
        let expected_occupied = self.expected_physical();

        // If there is already a winner, just do nothing.
        let game: &mut Game = self.game.as_mut().unwrap();
        if game.result().is_some() {
            return Ok(());
        }

        let diff = expected_occupied.get_different_bits(self.physical);
        if !diff.only_one_bit_set_to_one() {
            // If more than one bit differs - do nothing,
            // as there would be no way to determine what happens.
            // In this case the previous physical board state has to be restored before continuing.
            return Ok(());
        }

        match physical_board.0.cmp(&expected_occupied.0) {
            Greater => {
                // If more bits are set, a piece must have been placed.
                self.place_physical(Square::new(
                    expected_occupied
                        .get_different_bits(physical_board)
                        .first_one(),
                ));
            }
            Less => {
                // If fewer bits are set, a piece must have been removed.
                self.remove_physical(Square::new(
                    expected_occupied
                        .get_different_bits(physical_board)
                        .first_one(),
                ));
            }
            Equal => {
                // If the same number of bits are set, do nothing.
            }
        }

        Ok(())
    }

    pub fn get_state(&self) -> Option<ChessGameState> {
        if let Some(game) = &self.game {
            return Some(ChessGameState {
                physical: self.physical,
                expected_physical: self.expected_physical(),
                playing_state: self.playing_state,
                last_move: self.last_move(),
                possible_moves: self.get_possible_moves(),
                current_position: game.current_position(),
                active_player: game.side_to_move(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::chess_connector::LocalChessConnector;

    use super::*;

    #[test]
    fn test_tick_invalid_board() -> Result<(), ChessGameError> {
        let mut chess = ChessGame::new(LocalChessConnector::new()).unwrap();
        chess.reset("")?;

        let mut physical = chess.expected_physical();

        // Set initally correct
        chess.tick(physical)?;
        let initially_expected = chess.expected_physical();
        assert!(initially_expected == physical);

        // Take two black that shouldn't be taken
        physical = physical
            ^ BitBoard::from_square(Square::make_square(Rank::Eighth, File::A))
            ^ BitBoard::from_square(Square::make_square(Rank::Eighth, File::B));
        chess.tick(physical)?;
        let expected = chess.expected_physical();
        println!("{:?}", chess);
        assert!(expected == initially_expected);

        // Now take a2 - it should not try to make the move!
        physical = physical ^ BitBoard::from_square(Square::make_square(Rank::Second, File::A));
        chess.tick(physical)?;
        let expected = chess.expected_physical();
        println!("{:?}", chess);
        assert!(expected == initially_expected);

        // Try to place on a3
        physical = physical | BitBoard::from_square(Square::make_square(Rank::Third, File::A));
        chess.tick(physical)?;
        let expected = chess.expected_physical();
        println!("{:?}", chess);
        assert!(expected == initially_expected);

        // Until now - no real move was done -> expected is still the initially expected.

        Ok(())
    }
}
