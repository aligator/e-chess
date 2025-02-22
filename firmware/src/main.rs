use anyhow::Result;
use board::Board;
use chess_game::bitboard_extensions::*;
use chess_game::game::ChessGame;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use log::*;
use std::thread::sleep;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;

/// Entry point to our application.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    info!("Starting E-Chess!");

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mcp23017: I2cDriver<'_> = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let mut chess: ChessGame = ChessGame::new();

    let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio23)?;

    let mut board = Board::new(mcp23017, 0x20);
    board.setup()?;

    let mut display = display::Display::new(ws2812);
    display.setup()?;

    loop {
        match board.tick() {
            Ok(physical) => {
                let expected = chess.tick(physical);
                display.tick(physical, &chess)?;
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        sleep(Duration::from_millis(100));
    }
}
