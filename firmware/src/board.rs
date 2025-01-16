use chess::{BitBoard, Square};
use chess_game::bitboard_extensions::BitBoardExtensions;
use esp_idf_hal::{
    gpio::{AnyIOPin, Level, PinDriver, Pull},
    peripheral::Peripheral,
};

pub struct Board<'a, const N: usize> {
    column_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Output>; N],
    row_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Input>; N],

    field: BitBoard,
}

impl<'a, const N: usize> Board<'a, N> {
    pub fn new(
        column_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
        row_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
    ) -> Self {
        Board {
            column_pins: column_pins.map(|pin| PinDriver::output(pin).unwrap()),
            row_pins: row_pins.map(|pin| PinDriver::input(pin).unwrap()),

            field: BitBoard::new(0),
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
            // The pin needs to be set to low to read the values.
            col_pin.set_low().unwrap();

            for (row, row_pin) in &mut self.row_pins.iter().enumerate() {
                let set = row_pin.get_level() == Level::Low;
                self.field.set(
                    Square::make_square(chess::Rank::from_index(row), chess::File::from_index(col)),
                    set,
                );
            }

            // Afterwards set it high again.
            col_pin.set_high().unwrap();
        }
    }

    /// Returns the current state as bitboard representation.
    pub fn bitboard(&self) -> BitBoard {
        self.field
    }
}
