#![deny(warnings)]

use anyhow::Result;
use chess_game::chess_connector::ChessConnector;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::log::EspLogger;
use log::info;
use std::time::Duration;

mod bluetooth;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("Starting Bluetooth bridge");

    let (connector, ble_runtime) =
        bluetooth::init_ble_server("E-Chess Server", Duration::from_secs(9999))?;

    let _ble_bridge = ble_runtime.spawn();

    let games = connector.find_open_games()?;
    for game in games {
        info!("Found open game: {:?}", game);
    }

    loop {
        FreeRtos::delay_ms(1000);
    }
}
