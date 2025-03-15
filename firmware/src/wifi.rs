use anyhow::Result;
use esp_idf_hal::io::Write;
use esp_idf_hal::reset;
use esp_idf_svc::nvs::NvsPartitionId;
use esp_idf_svc::wifi::{self, Configuration};
use esp_idf_svc::{
    http::{
        server::{self, EspHttpServer},
        Method,
    },
    wifi::EspWifi,
};
use log::*;
use maud::{html, PreEscaped, DOCTYPE};
use std::thread::{self, sleep};
use std::time::Duration;

use crate::storage::Storage;

struct WifiSettings {
    ssid: String,
    password: String,
}

struct AppSettings {
    api_token: String,
}

enum Event {
    WifiSettings(WifiSettings),
    AppSettings(AppSettings),
}

pub fn page(body: String) -> String {
    html!(
        (DOCTYPE)
        html {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";

                title { "E-Chess" }
                link rel="stylesheet" href="/styles.css" {}
            }
            body {
                // Header with title and menu
                div class="header" {
                    // Title on the left
                    h1 { "E-Chess" }
                    
                    // Menu for navigation on the right
                    div class="menu-container" {
                        div class="menu" {
                            a href="/game" class="menu-item" id="menu-game" { "Game" }
                            a href="/settings" class="menu-item" id="menu-settings" { "Settings" }
                        }
                    }
                    
                    // GitHub button in top right
                    a href="https://github.com/aligator/e-chess" target="_blank" class="github-button" {
                        span class="github-icon" {}
                    }
                }

                // Main content
                div class="content" {
                    (PreEscaped(body))
                }

                // Common scripts
                script {
                    (PreEscaped(r#"
                    document.addEventListener('DOMContentLoaded', function() {
                        // Set active menu item based on current page
                        const path = window.location.pathname;
                        if (path === '/game' || path === '/') {
                            document.getElementById('menu-game').classList.add('active');
                        } else if (path === '/settings') {
                            document.getElementById('menu-settings').classList.add('active');
                        }
                    });
                    "#))
                }
            }
        }
    )
    .into_string()
}

pub fn register_wifi_settings<T: NvsPartitionId + 'static>(
    server: &mut EspHttpServer,
    mut wifi_driver: EspWifi<'static>,
    mut storage: Storage<T>,
) -> Result<()> {
    server.fn_handler("/settings", Method::Get, |request| {
        let html: String = page(
            html!(
                div class="container" {
                    p class="message" { 
                        "Please enter the SSID and password of the network you want to connect to." 
                    }
                    form action="/connect" method="POST" {
                        div class="form-group" {
                            label for="ssid" { "SSID:" }
                            input type="text" id="ssid" name="ssid" placeholder="Network name" {}
                        }
                        div class="form-group" {
                            label for="password" { "Password:" }
                            input type="password" id="password" name="password" placeholder="Network password" {}
                        }
                        input type="submit" value="Connect" {}
                    }
                }
                div class="container" {
                    p class="message" {
                        "Set here the leechess api token." 
                    }
                    form action="/save_settings" method="POST" {
                        div class="form-group" {
                            label for="api_token" { "API Token:" }
                            input type="text" id="api_token" name="api_token" placeholder="API Token" maxlength="24" {}
                        }
                            input type="submit" value="Save" {}
                    }
                }
            )
            .into_string(),
        );
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    let (tx, rx) = std::sync::mpsc::channel();

    let tx_wifi = tx.clone();
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
            let _ = tx_wifi.send(Event::WifiSettings(WifiSettings { ssid, password }));

            // Return success page
            let html = page(
                html!(
                    div class="container" {
                        p class="message" { "WiFi Settings Saved" }
                        p class="message" {
                            "Your device will now attempt to connect to the network."
                        }
                    }
                )
                .into_string(),
            );

            request.into_ok_response()?.write_all(html.as_bytes())
        } else {
            // Return error page
            let html = page(
                html!(
                    div class="container" {
                        p class="message error" { "Both SSID and password are required." }
                        a href="/settings" { "Go back" }
                    }
                )
                .into_string(),
            );
            request.into_ok_response()?.write_all(html.as_bytes())
        }
    })?;

    let tx_settings = tx.clone();
    server.fn_handler("/save_settings", Method::Post, move |mut request| {
        // Read POST body
        let mut buf = [0u8; 1024];
        let size = request.read(&mut buf)?;
        let body = std::str::from_utf8(&buf[..size]).expect("invalid body on /save_settings");
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
        let mut api_token = String::new();
        for (key, value) in params {
            match key {
                "api_token" => {
                    api_token = urlencoding::decode(value)
                        .map(|s| s.into_owned())
                        .unwrap_or_default()
                }
                _ => {}
            }
        }
        // Save api token
        let _ = tx_settings.send(Event::AppSettings(AppSettings { api_token }));

        // Return success page
        let html = page(
            html!(
                div class="container" {
                    p class="message" { "App Settings Saved" }
                }
            )
            .into_string(),
        );

        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    thread::spawn(move || {
        // Wait for events from the handler
        match rx.recv() {
            Ok(event) => match event {
                Event::WifiSettings(settings) => {
                    let config = wifi::Configuration::Client(wifi::ClientConfiguration {
                        ssid: heapless::String::try_from(settings.ssid.as_str()).unwrap(),
                        password: heapless::String::try_from(settings.password.as_str()).unwrap(),
                        ..Default::default()
                    });

                    info!("Received new config - restart wifi");

                    wifi_driver
                        .set_configuration(&config)
                        .expect("Failed to set configuration");
                    reset::restart();
                }
                Event::AppSettings(settings) => {
                    info!("Received new api token: {}", settings.api_token);
                    storage.set_str("api_token", &settings.api_token).unwrap();
                    reset::restart();
                }
            },
            Err(_) => {
                info!("No credentials received");
            }
        }
    });

    Ok(())
}

fn ap_config() -> wifi::Configuration {
    wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
        ssid: heapless::String::try_from("E-Chess").unwrap(),
        password: heapless::String::try_from("1337_e-chess").unwrap(),
        channel: 1,
        max_connections: 4,
        ..Default::default()
    })
}

fn try_connect(wifi_driver: &mut EspWifi) -> Result<()> {
    info!("Trying to connect to Wifi");
    wifi_driver.connect()?;

    let mut count = 0;
    while !wifi_driver.is_connected()? {
        if count > 30 {
            info!("Failed to connect to Wifi");
            info!("Starting Access Point while preserving settings");

            // Get current configuration and extract client config
            let current_config = wifi_driver.get_configuration()?;
            let client_config = if let Configuration::Client(conf) = current_config {
                conf
            } else {
                return Ok(());
            };

            // Create mixed configuration
            let ap_config = if let Configuration::AccessPoint(conf) = ap_config() {
                conf
            } else {
                return Ok(());
            };

            // Set mixed configuration
            let config = wifi::Configuration::Mixed(client_config, ap_config);
            wifi_driver.set_configuration(&config)?;
            wifi_driver.start()?;

            return Ok(());
        }

        info!("Waiting for Wifi connection...");
        sleep(Duration::from_secs(1));
        count += 1;
    }

    if wifi_driver.is_connected()? {
        info!("Connected to Wifi");
    } else {
        info!("Failed to connect to Wifi, enabled Access Point while preserving settings");
    }

    Ok(())
}

pub fn start_wifi<T: NvsPartitionId + 'static>(
    mut wifi_driver: EspWifi<'static>,
    storage: Storage<T>,
) -> Result<EspHttpServer<'static>> {
    let wifi_configuration: wifi::Configuration = match wifi_driver.get_configuration() {
        Ok(config) => {
            info!("Current Configuration: {:?}", config);
            config
        }
        Err(_) => {
            info!("No Configuration found, creating new one");
            let config = ap_config();
            wifi_driver.set_configuration(&config)?;
            config
        }
    };

    wifi_driver.start()?;

    if let Some(client_config) = wifi_configuration.as_client_conf_ref() {
        info!("Starting Client {}", client_config.ssid);
        try_connect(&mut wifi_driver)?;

        // Wait until dns is available.
        loop {
            let dns_res = wifi_driver.sta_netif().get_dns();
            info!("DNS: {:?}", dns_res);
            if !dns_res.is_unspecified() {
                break;
            }
            info!("Waiting for DNS...");

            sleep(Duration::from_secs(1));
        }
    } else if let Some(ap_config) = wifi_configuration.as_ap_conf_ref() {
        info!("Starting Access Point {}", ap_config.ssid);
        info!("IP info: {:?}", wifi_driver.ap_netif().get_ip_info());
    } else {
        info!("Unknown Wifi Configuration");
    }

    let mut server = EspHttpServer::new(&server::Configuration::default())?;

    register_wifi_settings(&mut server, wifi_driver, storage)?;

    Ok(server)
}
