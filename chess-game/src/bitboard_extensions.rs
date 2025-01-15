use chess::BitBoard;

pub trait BitBoardExtensions {
    fn only_one_bit_set_to_one(self) -> bool;
    fn get_different_bits(self, other: BitBoard) -> BitBoard;
    fn first_one(&self) -> u8;
}

impl BitBoardExtensions for BitBoard {
    fn only_one_bit_set_to_one(self) -> bool {
        self.0 != 0 && (self.0 & (self.0 - 1)) == 0
    }

    fn get_different_bits(self, other: BitBoard) -> BitBoard {
        return self ^ other;
    }

    fn first_one(&self) -> u8 {
        self.0.trailing_zeros() as u8
    }
}
