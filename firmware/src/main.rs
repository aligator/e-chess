use anyhow::Result;
use board::Board;
use chess_game::bitboard_extensions::*;
use chess_game::game::ChessGame;
use esp_idf_hal::i2c::*;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{self, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::ipv4::ClientConfiguration;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{self, EspWifi, WifiDriver};
use log::*;
use std::any::{Any, TypeId};
use std::thread::sleep;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;

struct WifiSettings {
    ssid: String,
    password: String,
}

fn start_wizard_server(wifi_driver: &mut EspWifi) -> Result<()> {
    let mut server = EspHttpServer::new(&server::Configuration::default())?;

    server.fn_handler("/", Method::Get, |request| {
        let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>E-Chess</title>
        </head>
        <body>
            <form action="/connect" method="POST">
            <label for="ssid">SSID:</label>
            <input type="text" id="ssid" name="ssid"><br><br>
            <label for="password">Password:</label>
            <input type="password" id="password" name="password"><br><br>
            <input type="submit" value="Connect">
            </form>
        </body>
        </html>
        "#;
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    let (tx, rx) = std::sync::mpsc::channel();
    let tx_clone = tx.clone();

    server.fn_handler("/connect", Method::Post, move |mut request| {
        // Read POST body
        let mut buf = [0u8; 1024];
        let size = request.read(&mut buf)?;
        let body = std::str::from_utf8(&buf[..size]).expect("invalid body on /connect");

        // Parse form data
        let params: Vec<(&str, &str)> = body
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key, value)),
                    _ => None,
                }
            })
            .collect();

        let mut ssid = String::new();
        let mut password = String::new();

        for (key, value) in params {
            match key {
                "ssid" => {
                    ssid = urlencoding::decode(value)
                        .map(|s| s.into_owned())
                        .unwrap_or_default()
                }
                "password" => {
                    password = urlencoding::decode(value)
                        .map(|s| s.into_owned())
                        .unwrap_or_default()
                }
                _ => {}
            }
        }

        if !ssid.is_empty() && !password.is_empty() {
            // Send credentials through channel
            let _ = tx_clone.send(WifiSettings { ssid, password });

            // Return success page
            let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Success</title>
            </head>
            <body>
                <h1>WiFi Settings Saved</h1>
                <p>Your device will now attempt to connect to the network.</p>
            </body>
            </html>
            "#;
            request.into_ok_response()?.write_all(html.as_bytes())
        } else {
            // Return error page
            let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Error</title>
            </head>
            <body>
                <h1>Error</h1>
                <p>Both SSID and password are required.</p>
                <a href="/">Go back</a>
            </body>
            </html>
            "#;
            request.into_ok_response()?.write_all(html.as_bytes())
        }
    })?;
    // Wait for credentials from the handler
    match rx.recv() {
        Ok(settings) => {
            let config = wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid: heapless::String::try_from(settings.ssid.as_str()).unwrap(),
                password: heapless::String::try_from(settings.password.as_str()).unwrap(),
                ..Default::default()
            });
            wifi_driver.stop()?;
            wifi_driver.set_configuration(&config)?;
            wifi_driver.start()?;
            wifi_driver.connect()?;
            while !wifi_driver.is_connected()? {
                info!("Connecting to Wifi {}", settings.ssid);
                sleep(Duration::from_secs(1));
            }
            info!("Connected to Wifi {}", settings.ssid);
        }
        Err(_) => {
            info!("No credentials received");
        }
    }

    Ok(())
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

    let nvs = EspDefaultNvsPartition::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs))?;

    let wifi_configuration: wifi::Configuration = match wifi_driver.get_configuration() {
        Ok(config) => {
            info!("Current Configuration: {:?}", config);
            config
        }
        Err(_) => {
            info!("No Configuration found, creating new one");
            let config = wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
                ssid: "E-Chess".try_into().unwrap(),
                password: heapless::String::try_from("1337_aligator").unwrap(),
                auth_method: wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            wifi_driver.set_configuration(&config)?;
            config
        }
    };

    wifi_driver.start()?;

    if let Some(client_config) = wifi_configuration.as_client_conf_ref() {
        info!("Connecting to Wifi {}", client_config.ssid);
        wifi_driver.connect()?;
        info!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info());

        while !wifi_driver.is_connected()? {
            sleep(Duration::from_secs(1));
        }

        info!("Connected to Wifi {}", client_config.ssid);
    } else if let Some(ap_config) = wifi_configuration.as_ap_conf_ref() {
        info!("Starting Access Point {}", ap_config.ssid);
        info!("IP info: {:?}", wifi_driver.ap_netif().get_ip_info());

        start_wizard_server(&mut wifi_driver)?;
    } else {
        info!("Unknown Wifi Configuration");
    }

    loop {
        sleep(Duration::from_secs(1));
    }
}
