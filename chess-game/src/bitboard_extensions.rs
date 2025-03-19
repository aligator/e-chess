use chess::{BitBoard, File, Rank, Square};

pub trait BitBoardExtensions {
    fn only_one_bit_set_to_one(self) -> bool;
    fn get_different_bits(self, other: BitBoard) -> BitBoard;
    fn first_one(&self) -> u8;
    fn get(self, square: Square) -> u8;
    fn set(&mut self, square: Square, value: bool);

    /// Print the bitboard as a chess board for debugging
    fn print(&self);
}

impl BitBoardExtensions for BitBoard {
    fn only_one_bit_set_to_one(self) -> bool {
        self.0 != 0 && (self.0 & (self.0 - 1)) == 0
    }

    fn get_different_bits(self, other: BitBoard) -> BitBoard {
        self ^ other
    }

    fn first_one(&self) -> u8 {
        self.0.trailing_zeros() as u8
    }

    fn get(self, square: Square) -> u8 {
        ((self.0 >> square.to_int()) & 1) as u8
    }

    fn set(&mut self, square: Square, value: bool) {
        if value {
            self.0 |= 1u64 << square.to_int();
        } else {
            self.0 &= !(1u64 << square.to_int());
        }
    }

    fn print(&self) {
        println!("\n   a b c d e f g h");
        println!("  ---------------");

        // Print board rows from top (rank 8) to bottom (rank 1)
        for rank in (0..8).rev() {
            print!("{} ", rank + 1); // Rank number
            for file in 0..8 {
                let square = Square::make_square(Rank::from_index(rank), File::from_index(file));
                let bit = BitBoard::from_square(square);
                let value = if (self & bit).0 != 0 { "1" } else { "0" };
                print!(" {}", value);
            }
            println!();
        }
        println!("  ---------------");
        println!("   a b c d e f g h");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_one_bit_set_to_one() {
        assert!(BitBoard::new(0b00100).only_one_bit_set_to_one());
        assert!(!BitBoard::new(0b100100).only_one_bit_set_to_one());
        assert!(!BitBoard::new(0b111111).only_one_bit_set_to_one());
        assert!(BitBoard::new(0b10000).only_one_bit_set_to_one());
        assert!(BitBoard::new(0b00001).only_one_bit_set_to_one());
    }
}
