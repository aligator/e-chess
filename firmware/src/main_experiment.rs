#![deny(warnings)]

use std::time::Duration;

use anyhow::Result;

use chess_game::requester::Requester;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::log::EspLogger;
use log::info;

use crate::bluetooth::Bluetooth;

mod bluetooth;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("Starting Bluetooth bridge");

    let connector = Bluetooth::create_and_spawn("E-Chess Server", Duration::from_secs(1000));

    loop {
        info!("Waiting for BLE connection...");
        FreeRtos::delay_ms(1000);
        if connector.is_connected() {
            FreeRtos::delay_ms(5000);
            info!("BLE connected");
            info!("Sending request ...");
            let data = connector.get("https://official-joke-api.appspot.com/random_joke")?;
            info!("Got response: {:?}", data);
        } else {
            info!("BLE NOT connected");
            continue;
        }
    }
}
