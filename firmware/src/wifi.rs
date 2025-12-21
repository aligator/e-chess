use anyhow::Result;
use esp_idf_hal::io::Write;
use esp_idf_hal::reset;
use esp_idf_svc::{
    http::{
        server::{self, EspHttpServer},
        Method,
    },
    wifi::{self, EspWifi},
};
use esp_ota::OtaUpdate;
use log::*;
use maud::{html, PreEscaped, DOCTYPE};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::Duration;

use crate::event::EventManager;
use crate::game::Settings;
use crate::Event;

struct WifiSettings {
    ssid: String,
    password: String,
}

struct AppSettings {
    api_token: String,
}

enum WifiEvent {
    WifiSettings(WifiSettings),
    AppSettings(AppSettings),
}

/// Information about the Access Point.
/// Can be used to display the SSID and password to the user.
#[derive(Debug, Clone)]
pub struct AccessPointInfo {
    pub ssid: String,
    pub password: String,
    pub ip: String,
}

/// Information about the current Wifi connection.
/// It does not contain the password as it should not be exposed after setup due to security reasons.
#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub ssid: String,
    pub ip: String,
}

#[derive(Debug, Clone)]
pub enum ConnectionStateEvent {
    NotConnected,
    AccessPoint(AccessPointInfo),
    Wifi(WifiInfo),
}

unsafe fn handle_favicon(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic("/favicon.ico", Method::Get, move |request| -> Result<()> {
        // Include the favicon file at compile time
        const FAVICON: &[u8] = include_bytes!("../assets/favicon.ico");

        let mut response = request.into_ok_response()?;
        response.write_all(FAVICON)?;
        Ok(())
    })?;
    Ok(())
}

unsafe fn handle_css(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic("/styles.css", Method::Get, move |request| -> Result<()> {
        // Include the CSS file at compile time
        const CSS: &[u8] = include_bytes!("../assets/styles.css");

        let mut response = request.into_response(200, None, &[("Content-Type", "text/css")])?;
        response.write_all(CSS)?;
        Ok(())
    })?;
    Ok(())
}

unsafe fn handle_firmware_upload(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic(
        "/upload-firmware",
        Method::Post,
        move |mut request| -> Result<()> {
            // Initialize OTA update
            let mut ota = OtaUpdate::begin()?;

            // Stream the firmware data in chunks
            let mut buffer = [0u8; 16]; // with bigger chunks it seems to be unstable...
            let mut total_bytes = 0;

            loop {
                let bytes_read = request.read(&mut buffer)?;
                if bytes_read == 0 {
                    info!("end of stream");
                    break; // End of stream
                }

                // Write the chunk to OTA
                ota.write(&buffer[..bytes_read])?;
                total_bytes += bytes_read;
            }

            // Finalize the update
            let mut completed_ota = ota.finalize()?;

            // Set the new partition as bootable
            completed_ota.set_as_boot_partition()?;

            let mut response = request.into_ok_response()?;
            response.write_all(
                format!(
                    "Firmware update successful ({} bytes). Restarting...",
                    total_bytes
                )
                .as_bytes(),
            )?;

            // Schedule a restart after a short delay
            thread::spawn(|| {
                thread::sleep(Duration::from_secs(2));
                unsafe {
                    esp_idf_sys::esp_restart();
                }
            });

            Ok(())
        },
    )?;
    Ok(())
}

unsafe fn handle_firmware_js(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic("/firmware.js", Method::Get, move |request| -> Result<()> {
        // Include the JavaScript file at compile time
        const JS: &[u8] = include_bytes!("../assets/firmware.js");

        let mut response =
            request.into_response(200, None, &[("Content-Type", "application/javascript")])?;
        response.write_all(JS)?;
        Ok(())
    })?;
    Ok(())
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

pub fn handle_main(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler("/", Method::Get, |request| {
        let html = page(
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
    Ok(())
}

pub fn handle_wifi_settings(
    server: &mut EspHttpServer,
    mut wifi_driver: EspWifi<'static>,
    settings: Arc<Mutex<Settings>>,
    tx_event: Sender<Event>,
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
                        "Set here the lichess api token."
                    }
                    p class="message" {
                        "You can generate a token "
                        a href="https://lichess.org/account/oauth/token/create?scopes[]=follow:read&scopes[]=challenge:read&scopes[]=challenge:write&scopes[]=board:play&description=E-Chess+board" target="_blank" { "here" }
                    }
                    form action="/save_settings" method="POST" {
                        div class="form-group" {
                            label for="api_token" { "API Token:" }
                            input type="text" id="api_token" name="api_token" placeholder="API Token" maxlength="24" {}
                        }
                        input type="submit" value="Save" {}
                    }
                }
                div class="container" {
                    p class="message" {
                        "Firmware Update"
                    }
                    div class="form-group" {
                        label for="firmware-upload" { "Select firmware file:" }
                        input type="file" id="firmware-upload" accept=".bin" onchange="uploadFirmware(this.files[0])" {}
                    }
                }
                script src="/firmware.js" {}
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
            let _ = tx_wifi.send(WifiEvent::WifiSettings(WifiSettings { ssid, password }));

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
        let _ = tx_settings.send(WifiEvent::AppSettings(AppSettings { api_token }));

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
                WifiEvent::WifiSettings(settings) => {
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
                WifiEvent::AppSettings(app_settings) => {
                    info!("Received new api token: {}", app_settings.api_token);
                    let mut settings = settings.lock().unwrap();
                    settings.token = app_settings.api_token.clone();

                    settings.save().unwrap();

                    // For now simply restart to apply new settings.
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
        auth_method: wifi::AuthMethod::WPA2WPA3Personal,
        channel: 3,
        max_connections: 4,
        ..Default::default()
    })
}

fn try_connect(wifi_driver: &mut EspWifi) -> Result<bool> {
    info!("Trying to connect to Wifi");
    wifi_driver.connect()?;

    let start_time = std::time::Instant::now();
    while !wifi_driver.is_connected()? {
        if std::time::Instant::now()
            .duration_since(start_time)
            .as_secs()
            > 30
        {
            info!("Failed to connect to Wifi");
            info!("Starting Access Point while preserving settings");
            let ap_config = ap_config();
            let wifi_config = wifi_driver.get_configuration()?;
            let mixed = wifi::Configuration::Mixed(
                wifi_config.as_client_conf_ref().unwrap().clone(),
                ap_config.as_ap_conf_ref().unwrap().clone(),
            );
            wifi_driver.set_configuration(&mixed)?;
            wifi_driver.start()?;

            return Ok(false);
        }

        if std::time::Instant::now()
            .duration_since(start_time)
            .as_nanos()
            % 2000000000
            == 0
        {
            info!("Still trying to connect...");
        }
        sleep(Duration::from_millis(500));
    }

    if wifi_driver.is_connected()? {
        info!("Connected to Wifi");
    } else {
        info!("Failed to connect to Wifi, enabled Access Point while preserving settings");
    }

    Ok(true)
}

pub fn start_wifi(
    event_manager: &EventManager<Event>,
    mut wifi_driver: EspWifi<'static>,
    settings: Arc<Mutex<Settings>>,
) -> Result<EspHttpServer<'static>> {
    let tx_event = event_manager.create_sender();

    let wifi_configuration: wifi::Configuration = match wifi_driver.get_configuration() {
        Ok(config) => {
            let default_config = ap_config();
            if let Some(client_config) = config.as_client_conf_ref() {
                info!(
                    "Valid Client Configuration found - use it: {}",
                    client_config.ssid
                );
                config
            } else {
                info!("No Client Configuration found. Fall back to AP.");
                // Always reset to the default AP config to ensure known credentials.
                wifi_driver.set_configuration(&default_config)?;
                default_config
            }
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
        let is_wifi = try_connect(&mut wifi_driver)?;

        if is_wifi {
            // Wait until dns is available.
            loop {
                let dns_res = wifi_driver.sta_netif().get_dns();
                let ip = wifi_driver.sta_netif().get_ip_info()?.ip.to_string();
                info!("DNS: {:?}", dns_res);
                info!("IP : {}", ip);
                if !dns_res.is_unspecified() && ip != "0.0.0.0" {
                    break;
                }
                info!("Waiting for IP and DNS...");

                sleep(Duration::from_secs(1));
            }
            tx_event.send(Event::ConnectionState(ConnectionStateEvent::Wifi(
                WifiInfo {
                    ssid: client_config.ssid.to_string(),
                    ip: wifi_driver.sta_netif().get_ip_info()?.ip.to_string(),
                },
            )))?;
        } else {
            // Display fallback AP info
            let config = ap_config();
            let ap_config = config.as_ap_conf_ref().unwrap();
            tx_event.send(Event::ConnectionState(ConnectionStateEvent::AccessPoint(
                AccessPointInfo {
                    ssid: ap_config.ssid.to_string(),
                    password: ap_config.password.to_string(),
                    ip: wifi_driver.ap_netif().get_ip_info()?.ip.to_string(),
                },
            )))?;
        }
    } else if let Some(ap_config) = wifi_configuration.as_ap_conf_ref() {
        info!("Starting Access Point {}", ap_config.ssid);
        info!("IP info: {:?}", wifi_driver.ap_netif());
        tx_event.send(Event::ConnectionState(ConnectionStateEvent::AccessPoint(
            AccessPointInfo {
                ssid: ap_config.ssid.to_string(),
                password: ap_config.password.to_string(),
                ip: wifi_driver.ap_netif().get_ip_info()?.ip.to_string(),
            },
        )))?;
    } else {
        info!("Unknown Wifi Configuration");
        let _ = tx_event.send(Event::ConnectionState(ConnectionStateEvent::NotConnected));
    }

    let mut server = EspHttpServer::new(&server::Configuration::default())?;

    unsafe {
        handle_favicon(&mut server)?;
        handle_css(&mut server)?;
        handle_firmware_js(&mut server)?;
        handle_firmware_upload(&mut server)?;
        handle_main(&mut server)?;
    }
    handle_wifi_settings(&mut server, wifi_driver, settings, tx_event)?;

    Ok(server)
}
