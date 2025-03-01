use anyhow::Result;
use esp_idf_hal::io::Write;
use esp_idf_hal::reset;
use esp_idf_svc::wifi::{self};
use esp_idf_svc::{
    http::{
        server::{self, EspHttpServer},
        Method,
    },
    wifi::EspWifi,
};
use log::*;
use maud::{html, PreEscaped};
use std::thread::{self, sleep};
use std::time::Duration;

struct WifiSettings {
    ssid: String,
    password: String,
}

pub fn page(body: String) -> String {
    html!(
        html {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "E-Chess" }
                style { r#"
                    body {
                        font-family: Arial, sans-serif;
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        background-color: #f0f0f0;
                        margin: 0;
                        padding: 20px;
                    }
                    h1 {
                        color: #333;
                        margin-bottom: 1em;
                    }
                    .container {
                        background-color: white;
                        padding: 30px;
                        border-radius: 8px;
                        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                        max-width: 400px;
                        width: 100%;
                    }
                    form {
                        display: flex;
                        flex-direction: column;
                        gap: 15px;
                    }
                    .form-group {
                        display: flex;
                        flex-direction: column;
                        gap: 5px;
                    }
                    label {
                        color: #2d3436;
                        font-weight: bold;
                    }
                    input[type="text"],
                    input[type="password"] {
                        padding: 10px;
                        border: 1px solid #ddd;
                        border-radius: 4px;
                        font-size: 16px;
                    }
                    input[type="submit"] {
                        background-color: #b58863;
                        color: white;
                        border: none;
                        padding: 12px;
                        border-radius: 4px;
                        font-size: 16px;
                        cursor: pointer;
                        margin-top: 10px;
                    }
                    input[type="submit"]:hover {
                        background-color: #9e7657;
                    }
                    .message {
                        text-align: center;
                        margin-bottom: 20px;
                        line-height: 1.5;
                    }
                    .error {
                        color: #d63031;
                    }
                    a {
                        color: #b58863;
                        text-decoration: none;
                    }
                    a:hover {
                        text-decoration: underline;
                    }
                "# }
            }
            body { (PreEscaped(body)) }
        }
    )
    .into_string()
}

pub fn register_wifi_settings(
    server: &mut EspHttpServer,
    mut wifi_driver: EspWifi<'static>,
) -> Result<()> {
    server.fn_handler("/settings", Method::Get, |request| {
        let html: String = page(
            html!(
                h1 { "E-Chess" }
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
            )
            .into_string(),
        );
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    let (tx, rx) = std::sync::mpsc::channel();

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
            let _ = tx.send(WifiSettings { ssid, password });

            // Return success page
            let html = page(
                html!(
                    h1 { "E-Chess" }
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
                    h1 { "E-Chess" }
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

    thread::spawn(move || {
        // Wait for events from the handler
        match rx.recv() {
            Ok(settings) => {
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
            info!("Starting Access Point");
            let config = ap_config();
            wifi_driver.set_configuration(&config)?;

            reset::restart();
        }

        info!("Waiting for Wifi connection...");
        sleep(Duration::from_secs(1));
        count += 1;
    }

    if wifi_driver.is_connected()? {
        info!("Connected to Wifi");
    } else {
        info!("Failed to connect to Wifi, enabled Accesspoint instead");
    }

    Ok(())
}

pub fn start_wifi(mut wifi_driver: EspWifi<'static>) -> Result<EspHttpServer<'static>> {
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
    } else if let Some(ap_config) = wifi_configuration.as_ap_conf_ref() {
        info!("Starting Access Point {}", ap_config.ssid);
        info!("IP info: {:?}", wifi_driver.ap_netif().get_ip_info());
    } else {
        info!("Unknown Wifi Configuration");
    }

    let mut server = EspHttpServer::new(&server::Configuration::default())?;

    register_wifi_settings(&mut server, wifi_driver)?;

    Ok(server)
}
