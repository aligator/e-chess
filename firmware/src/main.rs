use anyhow::Result;
use board::Board;
use chess_game::game::ChessGame;
use chess_game::lichess::LichessConnector;
use embedded_svc::http::Method;
use esp_idf_hal::i2c::*;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use log::*;
use maud::html;
use request::EspRequester;
use std::thread::sleep;
use std::time::Duration;
use storage::Storage;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;
mod request;
mod storage;
mod web;
mod wifi;

fn run_game(
    token: String,
    mcp23017: I2cDriver<'_>,
    ws2812: Ws2812Esp32Rmt,
    web: &mut web::Web,
) -> Result<()> {
    #[cfg(not(feature = "no_board"))]
    let mut board = Board::new(mcp23017, 0x20);
    #[cfg(not(feature = "no_board"))]
    board.setup()?;

    let mut display = display::Display::new(ws2812);
    display.setup()?;

    // Create a requester with the API key
    let requester = EspRequester::new(token.clone());
    let lichess_connector = LichessConnector::new(requester);

    // Use the game ID from the web interface
    let game_id = web.get_game_id();
    let mut chess = ChessGame::new(lichess_connector, &game_id)?;

    // Start the main loop
    info!("Start app loop");
    loop {
        // Check if the game ID has changed via the event channel
        if let Some(new_game_id) = web.check_for_game_id_change() {
            // Game ID has changed, reload the game
            info!("Game ID changed to {}, reloading game", new_game_id);

            // Create a new requester and connector
            let requester = EspRequester::new(token.clone());
            let lichess_connector = LichessConnector::new(requester);

            // Create a new chess game with the new ID
            match ChessGame::new(lichess_connector, &new_game_id) {
                Ok(new_chess) => {
                    chess = new_chess;
                    info!("Game reloaded successfully");
                }
                Err(e) => {
                    error!("Failed to load game: {:?}", e);
                    // Continue with the current game
                }
            }
        }

        #[cfg(not(feature = "no_board"))]
        match board.tick() {
            Ok(physical) => {
                let new_expected = chess.tick(physical);
                match new_expected {
                    Ok(expected) => {
                        web.tick(chess.game.clone());
                        display.tick(physical, &chess)?;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => return Err(e),
        }

        #[cfg(feature = "no_board")]
        {
            let game = chess::Game::default();
            let new_expected = chess.tick(*game.current_position().combined());
            match new_expected {
                Ok(expected) => {
                    web.tick(chess.game.clone());
                    display.tick(*game.current_position().combined(), &chess)?;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

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

    let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio23)?;

    let mut web = web::Web::new();

    let nvs = EspDefaultNvsPartition::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs.clone()))?;
    let storage = Storage::new(nvs.clone())?;

    let token = storage.get_str::<25>("api_token")?;
    info!("API Token: {:?}", token);

    let mut server = wifi::start_wifi(wifi_driver, storage)?;

    server.fn_handler("/", Method::Get, |request| {
        let html = wifi::page(
            html!(
                h1 { "E-Chess" }
                p { "Welcome to E-Chess!" }
                a href="/settings" { "Settings" }
                a href="/game" { "Game" }
            )
            .into_string(),
        );

        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    web.register(&mut server)?;

    if let Some(token) = token {
        match run_game(token, mcp23017, ws2812, &mut web) {
            Ok(_) => {
                warn!("Stopping game loop");
                Ok(())
            }
            Err(e) => {
                warn!("Stopping game loop due to error: {:?}", e);
                loop {
                    sleep(Duration::from_millis(1000));
                }
            }
        }
    } else {
        error!("No token found");
        Err(anyhow::anyhow!("No token found"))
    }
}
