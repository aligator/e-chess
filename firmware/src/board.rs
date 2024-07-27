use esp_idf_hal::{
    gpio::{AnyIOPin, Level, PinDriver, Pull},
    peripheral::Peripheral,
};

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
}
