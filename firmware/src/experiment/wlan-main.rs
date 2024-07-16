// use std::sync::mpsc::channel;
use std::result::Result::Ok;
use std::str;
use std::{ptr, string::String, thread, time::*};

use anyhow::*;
use log::*;

// Common IDF stuff
use esp_idf_hal::modem::*;
use esp_idf_hal::peripheral::*;
use esp_idf_hal::prelude::*;
use esp_idf_sys::link_patches;
use esp_idf_sys::time;
use esp_idf_sys::time_t;

// Wi-Fi
use esp_idf_svc::eventloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use esp_idf_svc::wifi::*;

use esp_idf_svc::log::EspLogger;

const WIFI_SSID: &str = "Wokwi-GUEST";
const WIFI_PASS: &str = "";

/// Entry point to our application.
///
/// It sets up a Wi-Fi connection to the Access Point given in the
/// configuration, then blinks the RGB LED green/blue.
///
/// If the LED goes solid red, then it was unable to connect to your Wi-Fi
/// network.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Set up peripherals and display
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!(
        "About to initialize WiFi (SSID: {}, PASS: {})",
        WIFI_SSID, WIFI_PASS
    );

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;

    connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS)?;

    info!("WIFI connection done");

    loop {}
}

fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    wifi_ssid: &str,
    wifi_password: &str,
) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.try_into().unwrap(),
        bssid: None,
        auth_method: esp_idf_svc::wifi::AuthMethod::None,
        password: wifi_password.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}
