use esp_idf_hal::{
    gpio::{AnyIOPin, Level, PinDriver, Pull},
    peripheral::Peripheral,
};

use crate::bitboard::set_bit;

pub struct Board<'a, const N: usize> {
    column_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Output>; N],
    row_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Input>; N],

    pub field: [[bool; N]; N],
}

impl<'a, const N: usize> Board<'a, N> {
    pub fn new(
        column_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
        row_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
    ) -> Self {
        Board {
            column_pins: column_pins.map(|pin| PinDriver::output(pin).unwrap()),
            row_pins: row_pins.map(|pin| PinDriver::input(pin).unwrap()),

            field: [[false; N]; N],
        }
    }

    pub fn size(&self) -> usize {
        N
    }

    pub fn setup(&mut self) {
        // Set up the pullup.
        for pin in &mut self.row_pins {
            pin.set_pull(Pull::Up).unwrap();
        }

        // Set all columns high.
        for pin in &mut self.column_pins {
            pin.set_high().unwrap();
        }
    }

    pub fn tick(&mut self) {
        // Check each field
        for (col, col_pin) in &mut self.column_pins.iter_mut().enumerate() {
            col_pin.set_low().unwrap();

            for (row, row_pin) in &mut self.row_pins.iter().enumerate() {
                self.field[row][col] = row_pin.get_level() == Level::Low;
            }

            col_pin.set_high().unwrap();
        }
    }

    /// Returns the current state as bitboard representation.
    /// For compatibility with the chess board later it resembles to
    /// a full u32 bitboard, but uses only the bottom right part of it.
    ///
    /// ```
    /// msb
    /// 00000000
    /// 00000000
    /// 00000000
    /// 00000000
    /// 00000000
    /// 00000111
    /// 00000111
    /// 00000111 lsb
    /// ```
    pub fn bitboard(&self) -> u32 {
        let mut bit_board: u32 = 0;

        for (row, columns) in self.field.iter().enumerate() {
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
}
