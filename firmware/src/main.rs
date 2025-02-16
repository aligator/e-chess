use anyhow::Result;
use board::Board;
use chess_game::bitboard_extensions::*;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use log::*;
use std::thread::sleep;
use std::time::Duration;

mod board;

/// Entry point to our application.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    info!("Starting io expander mcp23017 test!");

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mcp23017: I2cDriver<'_> = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let mut board = Board::new(mcp23017, 0x20);
    board.setup()?;

    loop {
        info!("Looping");

        match board.tick() {
            Ok(physical) => {
                physical._print();
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        sleep(Duration::from_millis(1000));
    }
}
