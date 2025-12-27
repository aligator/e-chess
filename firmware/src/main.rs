#![deny(warnings)]

use anyhow::Result;
use chess_game::requester::Requester;
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

    let (connector, ble_runtime, is_connected) =
        bluetooth::init_ble_server("E-Chess Server", Duration::from_secs(1000))?;

    let _ble_bridge = ble_runtime.spawn();

    loop {
        info!("Waiting for BLE connection...");
        FreeRtos::delay_ms(1000);
        if *is_connected.lock().unwrap() {
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
