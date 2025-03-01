use anyhow::Result;
use board::Board;
use chess_game::game::ChessGame;
use esp_idf_hal::i2c::*;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::Method;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use log::*;
use maud::html;
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;
mod wifi;

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

    let mut chess: ChessGame = ChessGame::default();

    let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio23)?;

    #[cfg(not(feature = "no_board"))]
    let mut board = Board::new(mcp23017, 0x20);
    #[cfg(not(feature = "no_board"))]
    board.setup()?;

    let mut display = display::Display::new(ws2812);
    display.setup()?;

    let (game_tx, game_rx) = mpsc::channel::<chess::Game>();

    info!("Spawn wifi thread");
    std::thread::spawn(|| -> anyhow::Result<()> {
        let nvs = EspDefaultNvsPartition::take()?;
        let sys_loop = EspSystemEventLoop::take()?;
        let wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;
        let mut server = wifi::start_wifi(wifi_driver)?;

        server.fn_handler("/", Method::Get, |request| {
            let html = wifi::page(
                html!(
                    h1 { "E-Chess" }
                    p { "Welcome to E-Chess!" }
                    p { "Please click the button below to configure the WiFi settings." }
                    form action="/settings" method="GET" {
                input type="submit" value="Configure WiFi" {}
                    }
                )
                .into_string(),
            );

            request.into_ok_response()?.write_all(html.as_bytes())
        })?;

        loop {
            sleep(Duration::from_secs(1));
        }
    });

    info!("Start app loop");
    loop {
        #[cfg(not(feature = "no_board"))]
        match board.tick() {
            Ok(physical) => {
                let _ = chess.tick(physical);
                game_tx.send(chess.game.clone())?;
                display.tick(physical, &chess)?;
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        sleep(Duration::from_millis(100));
    }
}
