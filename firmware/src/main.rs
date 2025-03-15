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
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use log::*;
use maud::html;
use request::EspRequester;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use storage::Storage;
use web::{GameCommandEvent, GameStateEvent};
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
    mut server: &mut EspHttpServer<'static>,
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
    let mut game = ChessGame::new(lichess_connector)?;
    info!("Created ChessGame");

    let web = web::Web::new();
    let (state_tx, state_rx) = channel::<GameStateEvent>();
    info!("Created state channel: {:?}", state_tx);
    let command_rx = web.register(&mut server, state_rx)?;
    info!("Registered web interface");

    // Start the main loop
    info!("Start app loop");
    loop {
        // Check if event happens
        if let Ok(event) = command_rx.try_recv() {
            info!("Received command event: {:?}", event);
            match event {
                GameCommandEvent::LoadNewGame(game_id) => {
                    info!("Loading new game: {}", game_id);

                    // Load the new game
                    match game.reset(&game_id) {
                        Ok(_) => {
                            info!("Successfully reset game with ID: {}", game_id);
                            // Notify the UI about the new game
                            match state_tx.send(GameStateEvent::UpdateGame(game.game.clone())) {
                                Ok(_) => info!("Sent game update event (new game)"),
                                Err(e) => warn!("Failed to send game update event: {:?}", e),
                            }

                            // Send the GameLoaded event with the game ID
                            match state_tx.send(GameStateEvent::GameLoaded(game_id.clone())) {
                                Ok(_) => info!("Sent game loaded event for ID: {}", game_id),
                                Err(e) => warn!("Failed to send game loaded event: {:?}", e),
                            }
                        }
                        Err(e) => {
                            warn!("Failed to reset game: {:?}", e);

                            // Send an empty GameLoaded event to indicate failure
                            match state_tx.send(GameStateEvent::GameLoaded(String::new())) {
                                Ok(_) => info!("Sent empty game loaded event to indicate failure"),
                                Err(e) => warn!("Failed to send game loaded event: {:?}", e),
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "no_board"))]
        {
            if let Some(_) = game.game {
                match board.tick() {
                    Ok(physical) => {
                        match game.tick(physical) {
                            Ok(_expected) => {
                                // TODO: not sure if this isn't a bit inefficient to do every tick...
                                match state_tx.send(GameStateEvent::UpdateGame(game.game.clone())) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        warn!("Failed to send game update: {:?}", e);
                                    }
                                }
                                display.tick(physical, &game)?;
                            }
                            Err(e) => {
                                warn!("Error in game tick: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error in board tick: {:?}", e);
                    }
                }
            }
        }

        #[cfg(feature = "no_board")]
        {
            if let Some(_) = game.game.as_ref() {
                let new_expected =
                    game.tick(*game.game.as_ref().unwrap().current_position().combined());
                match new_expected {
                    Ok(_expected) => {
                        match state_tx.send(GameStateEvent::UpdateGame(game.game.clone())) {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Failed to send game update: {:?}", e);
                            }
                        }
                        display.tick(
                            *game.game.as_ref().unwrap().current_position().combined(),
                            &game,
                        )?;
                    }
                    Err(e) => {
                        warn!("Error in game tick: {:?}", e);
                    }
                }
            }
        }

        // Sleep to reduce CPU usage and make logs more readable
        sleep(Duration::from_millis(500));
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

    if let Some(token) = token {
        match run_game(token, mcp23017, ws2812, &mut server) {
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
