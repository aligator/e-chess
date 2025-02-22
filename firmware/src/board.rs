use anyhow::Result;
use chess::BitBoard;
use esp_idf_hal::{delay::BLOCK, i2c::*};

use crate::constants::BOARD_SIZE;

pub struct Board<'a> {
    i2c: I2cDriver<'a>,
    addr: u8,
}

impl<'a> Board<'a> {
    pub fn new(i2c: I2cDriver<'a>, addr: u8) -> Self {
        Self { i2c, addr }
    }

    pub fn setup(&mut self) -> Result<()> {
        // Configure GPA = input
        // Configure GPB = output
        let msg = &[0x00, 0xFF, 0x00];
        self.i2c.write(self.addr, msg, BLOCK)?;
        // Enable Pull ups for the inputs
        let pullup_msg = &[0x0C, 0xFF]; // 0x0D is GPPUB register, 0xFF enables pull-ups for all pins
        self.i2c.write(self.addr, pullup_msg, BLOCK)?;
        Ok(())
    }

    pub fn tick(&mut self) -> Result<BitBoard> {
        let mut board: u64 = 0;

        for col in 0..BOARD_SIZE {
            // Set the col LOW that should be read.
            // Set all other cols HIGH.
            let enable_col = &[0x13, !(0x1 << col)];
            self.i2c.write(self.addr, enable_col, BLOCK)?;

            // Set register pointer to GPIOA (0x12)
            self.i2c.write(self.addr, &[0x12], BLOCK)?;

            // Read from Port A (inputs)
            let mut row_data = [0u8; 1];
            self.i2c.read(self.addr, &mut row_data, BLOCK)?;

            // Add row to the board.
            // Move row data to the correct position using bitshift.
            board |= (!row_data[0] as u64) << (col * BOARD_SIZE);
        }

        Ok(BitBoard::new(board))
    }
}
