use anyhow::Result;
use chess::BitBoard;
use esp_idf_hal::{delay::BLOCK, i2c::*};

use crate::constants::BOARD_SIZE;

pub trait Board {
    fn setup(&mut self) -> Result<()>;
    fn tick(&mut self, last_physical: BitBoard) -> Result<BitBoard>;
}

pub struct MCP23017Board<'a> {
    i2c: I2cDriver<'a>,
    addr: u8,
}
impl<'a> MCP23017Board<'a> {
    pub fn new(i2c: I2cDriver<'a>, addr: u8) -> Self {
        Self { i2c, addr }
    }
}

impl<'a> Board for MCP23017Board<'a> {
    fn setup(&mut self) -> Result<()> {
        // Configure GPA = input
        // Configure GPB = output
        let msg = &[0x00, 0xFF, 0x00];
        self.i2c.write(self.addr, msg, BLOCK).expect("Failed to write to MCP23017. You may use the feature 'no_board' for debugging the app without a real board");
        // Enable Pull ups for the inputs
        let pullup_msg = &[0x0C, 0xFF]; // 0x0D is GPPUB register, 0xFF enables pull-ups for all pins
        self.i2c.write(self.addr, pullup_msg, BLOCK)?;
        Ok(())
    }

    fn tick(&mut self, _last_physical: BitBoard) -> Result<BitBoard> {
        let mut board: u64 = 0;

        for col in 0..BOARD_SIZE {
            // Set the col LOW that should be read.
            // Set all other cols HIGH.
            let enable_col = &[0x13, !(0x1 << col)];
            self.i2c
                .write(self.addr, enable_col, BLOCK)
                .map_err(|err| anyhow::format_err!("set all high {}", err))?;

            // Set register pointer to GPIOA (0x12)
            self.i2c
                .write(self.addr, &[0x12], BLOCK)
                .map_err(|err| anyhow::format_err!("set low {}", err))?;

            // Read from Port A (inputs)
            let mut col_data = [0u8; 1];
            self.i2c
                .read(self.addr, &mut col_data, BLOCK)
                .map_err(|err| anyhow::format_err!("read {}", err))?;
            let column = !col_data[0] as u64;
            // Shift the column data to the correct position.
            board |= ((column & 0b00000001)
                | ((column & 0b00000010) << 7)
                | ((column & 0b00000100) << 14)
                | ((column & 0b00001000) << 21)
                | ((column & 0b00010000) << 28)
                | ((column & 0b00100000) << 35)
                | ((column & 0b01000000) << 42)
                | ((column & 0b10000000) << 49))
                << (col);
        }

        Ok(BitBoard::new(board))
    }
}

#[cfg(feature = "no_board")]
pub struct FakeBoard {}

#[cfg(feature = "no_board")]
impl Board for FakeBoard {
    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn tick(&mut self, last_physical: BitBoard) -> Result<BitBoard> {
        Ok(last_physical)
    }
}
