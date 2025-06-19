use anyhow::Result;
use board::Board;
use chess::BitBoard;
use chess_game::game::ChessGameState;
use embedded_svc::http::Method;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::rmt::TxRmtDriver;
use esp_idf_hal::{i2c::*, rmt::config::TransmitConfig};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use game::GameStateEvent;
use log::*;
use maud::html;
use std::thread::sleep;
use std::time::Duration;
use storage::Storage;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::event::EventManager;
use crate::game::GameCommandEvent;

mod board;
mod constants;
mod display;
mod event;
mod game;
mod request;
mod storage;
mod web;
mod wifi;

#[derive(Debug, Clone)]
enum Event {
    GameState(GameStateEvent),
    GameCommand(GameCommandEvent),
}

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

fn run_game(
    token: Option<String>,
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

    // Load standard local game initially
    let event_manager = EventManager::<Event>::new();
    event_manager.start_thread();

    let settings = game::Settings {
        token: token.unwrap_or_default(),
    };

    let web = web::Web::new();
    web.register(&mut server, &event_manager)?;
    info!("Registered web interface");

    game::run_game(settings, &event_manager);

    // Start the main loop
    info!("Start app loop");
    let rx = event_manager.create_receiver();

    let tx = event_manager.create_sender();
    let mut last_physical = BitBoard::new(0);
    let mut last_game_state: Option<ChessGameState> = None;
    loop {
        // Tick the physical board
        let physical = board.tick()?;
        if physical != last_physical {
            last_physical = physical;
            if let Err(e) = tx.send(Event::GameCommand(GameCommandEvent::UpdatePhysical(
                physical,
            ))) {
                warn!("Failed to send update physical event: {:?}", e);
            }
        }

        // Check if event happens
        if let Ok(event) = rx.try_recv() {
            info!("Received event: {:?}", event);
            match event {
                Event::GameState(game_state_event) => match game_state_event {
                    GameStateEvent::UpdateGame(_expected_physical, game_state) => {
                        last_game_state = Some(game_state);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(game_state) = last_game_state.clone() {
            display.tick(physical, &game_state)?;
        }

        // #[cfg(not(feature = "no_board"))]
        // {
        //     if let Some(_) = chess_game.game() {
        //         match board.tick() {
        //             Ok(physical) => {
        //                 match chess_game.tick(physical) {
        //                     Ok(_expected) => {
        //                         // TODO: not sure if this isn't a bit inefficient to do every tick...
        //                         match state_tx.send(GameStateEvent::UpdateGame(chess_game.game())) {
        //                             Ok(_) => {}
        //                             Err(e) => {
        //                                 warn!("Failed to send game update: {:?}", e);
        //                             }
        //                         }
        //                         display.tick(physical, &chess_game)?;
        //                     }
        //                     Err(e) => {
        //                         warn!("Error in game tick: {:?}", e);
        //                     }
        //                 }
        //             }
        //             Err(e) => {
        //                 warn!("Error in board tick: {:?}", e);
        //             }
        //         }
        //     }
        // }

        // #[cfg(feature = "no_board")]
        // {
        //     if let Some(game) = chess_game.game() {
        //         let new_expected = chess_game.tick(*game.clone().current_position().combined());
        //         match new_expected {
        //             Ok(_expected) => {
        //                 match state_tx.send(GameStateEvent::UpdateGame(chess_game.game())) {
        //                     Ok(_) => {}
        //                     Err(e) => {
        //                         warn!("Failed to send game update: {:?}", e);
        //                     }
        //                 }
        //                 display.tick(*game.clone().current_position().combined(), &chess_game)?;
        //             }
        //             Err(e) => {
        //                 warn!("Error in game tick: {:?}", e);
        //             }
        //         }
        //     }
        // }

        // Sleep to reduce CPU usage
        sleep(Duration::from_millis(100));
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
    #[cfg(esp32)]
    info!("Running on ESP32");
    #[cfg(esp32s3)]
    info!("Running on ESP32S3");

    #[cfg(esp32)]
    let sda = peripherals.pins.gpio21;
    #[cfg(esp32s3)]
    let sda = peripherals.pins.gpio8;

    #[cfg(esp32)]
    let scl = peripherals.pins.gpio22;
    #[cfg(esp32s3)]
    let scl = peripherals.pins.gpio9;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mcp23017: I2cDriver<'_> = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let driver_config = TransmitConfig::new()
        .clock_divider(1) // Required parameter.
        .mem_block_num(8); // Increase the number depending on your code.

    #[cfg(esp32)]
    let driver = TxRmtDriver::new(
        peripherals.rmt.channel0,
        peripherals.pins.gpio23,
        &driver_config,
    )?;
    #[cfg(esp32s3)]
    let driver = TxRmtDriver::new(
        peripherals.rmt.channel0,
        peripherals.pins.gpio4,
        &driver_config,
    )?;

    let ws2812 = Ws2812Esp32Rmt::new_with_rmt_driver(driver)?;

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
}
