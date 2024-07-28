use log::*;

use crate::bitboard::*;

#[derive(Default, Clone, Copy)]
/// Defines a "snapshot" of the game.
/// It contains the board state, so it
/// can be used to roll back changes.
pub struct HistoryEntry {
    // Use bitboards here.
    // This makes it very nice to test all possible win conditions
    // And to manipulate the state by using bit operations
    //
    /// The pieces of each player respectively.
    pub players: [u32; 2],

    /// If there is a winner its index is saved here.
    pub winner: Option<usize>,
}

impl HistoryEntry {
    fn occupied(self) -> u32 {
        self.players[0] | self.players[1]
    }
}

pub struct GameState {
    pub board: HistoryEntry,
    pub _player: usize,
}

pub(crate) struct TicTacToe<const N: usize> {
    // TicTacToe has a fixed count of possible history entries.
    // So no need for a dynamic data structure.
    //
    /// Contains the full game history.
    /// It should contain Some state up to the current index.
    /// The first element should always contain the initial state.
    history: [Option<HistoryEntry>; 10],

    /// The current index in the history
    current_index: usize,
}

impl<const N: usize> Default for TicTacToe<N> {
    fn default() -> Self {
        Self {
            history: [
                Some(HistoryEntry {
                    players: [0, 0],
                    winner: None,
                }),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            current_index: 0,
        }
    }
}

const WINNING_MASKS: [u32; 8] = [
    // rows
    0b00000000_00000000_00000000_00000000_00000000_00000111_00000000_00000000,
    0b00000000_00000000_00000000_00000000_00000000_00000000_00000111_00000000,
    0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000111,
    // columns
    0b00000000_00000000_00000000_00000000_00000000_00000100_00000100_00000100,
    0b00000000_00000000_00000000_00000000_00000000_00000010_00000010_00000010,
    0b00000000_00000000_00000000_00000000_00000000_00000001_00000001_00000001,
    // diagonals
    0b00000000_00000000_00000000_00000000_00000000_00000100_00000010_00000001,
    0b00000000_00000000_00000000_00000000_00000000_00000001_00000010_00000100,
];

impl<const N: usize> TicTacToe<N> {
    pub fn new() -> Self {
        TicTacToe::default()
    }

    fn current(&self) -> HistoryEntry {
        self.history[self.current_index].expect("index not in the history")
    }

    fn current_player(&self) -> usize {
        self.current_index % 2
    }

    fn push(&mut self, new_state: HistoryEntry) {
        self.current_index += 1;
        self.history[self.current_index] = Some(new_state);
    }

    fn pull(&mut self) -> HistoryEntry {
        self.history[self.current_index] = None;
        self.current_index -= 1;
        return self.current();
    }

    pub fn tick(&mut self, now_occupied: u32) -> GameState {
        let state = self.current();

        let last_occupied = state.occupied();
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
        if last_occupied > now_occupied && self.current_index != 0 {
            let previous = self.pull();
            return GameState {
                board: previous,
                _player: self.current_player(),
            };
        } else if last_occupied == now_occupied
            || (last_occupied > now_occupied && self.current_index == 0)
        {
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
    }

    /// check the winning conditions.
    /// Sets the respective player as winner if needed.
    fn calculate_win(&self, state: &mut HistoryEntry) {
        for (player_index, player) in state.players.iter().enumerate() {
            for mask in WINNING_MASKS.iter() {
                if *player & *mask == *mask {
                    state.winner = Some(player_index);
                    return;
                }
            }
        }
    }
}
