use crate::bitboard::*;

pub(crate) struct TicTacToe<const N: usize> {
    // Use bitboards here.
    // This should make it very nice to test all possible win conditions
    // And to manipulate the state by using bit operations
    pub players: [u32; 2],
    current_player: usize,

    pub last_board: u32,

    pub winner: Option<usize>,
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

fn to_bit_board<const N: usize>(board: [[bool; N]; N]) -> u32 {
    // msb
    // 00000000
    // 00000000
    // 00000000
    // 00000000
    // 00000000
    // 00000111
    // 00000111
    // 00000111 lsb
    //
    // u32 is what a full chess game would have.
    // This introduces possible compatibility even if the actual board is bigger,
    // so that this ticTacToe can run on a full sized chess board, too.
    // As this is only ticTacToe, only the area marked with 1 is used.

    let mut bit_board: u32 = 0;

    for (row, columns) in board.iter().enumerate() {
        for (column, is_set) in columns.iter().enumerate() {
            if !is_set {
                continue;
            }

            //                                                    + Padding to the bigger u32 chess board
            let pos = (N - row - 1) * N + (N - column - 1) + (N - row - 1) * (8 - N);
            bit_board = set_bit(bit_board, pos);
        }
    }

    bit_board
}

impl<const N: usize> TicTacToe<N> {
    pub fn new() -> Self {
        TicTacToe {
            current_player: 0,
            last_board: 0,
            players: [0, 0],
            winner: None,
        }
    }

    pub fn tick(&mut self, occupied_fields: [[bool; N]; N]) {
        // Convert the board to a bit_board. (TODO: The board.rs could already just work with a bitboard...)
        let new_board = to_bit_board(occupied_fields);

        // If the new board is empty - reset the game.
        if new_board == 0 && self.last_board != 0 {
            self.current_player = 0;
            self.last_board = 0;
            self.players = [0, 0];
            self.winner = None;
        }

        // If there is already a winner, just do nothing.
        if self.winner.is_some() {
            return;
        }

        // The new board must have more bits set - e.g. it must be a higher number.
        if self.last_board >= new_board {
            return;
        }

        // First get all "different" fields.
        // Due to the check before, new bits can only come from the new_board.
        // Then only check if it is only 1 new bit. Else something must be wrong.
        let diff = only_different(new_board, self.last_board);
        let only_one = only_one_bit_set_to_one(diff);
        if !only_one {
            return;
        }

        // Add the new field to the current player.
        self.players[self.current_player] = self.players[self.current_player] | diff;
        self.current_player = if self.current_player == 1 { 0 } else { 1 };

        self.last_board = new_board;

        self.check();
    }

    /// check the winning conditions.
    /// Sets the respective player as winner if needed.
    fn check(&mut self) {
        for (player_index, player) in self.players.iter().enumerate() {
            for mask in WINNING_MASKS.iter() {
                if *player & *mask == *mask {
                    self.winner = Some(player_index);
                    return;
                }
            }
        }
    }
}
