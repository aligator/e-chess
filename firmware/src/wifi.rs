use anyhow::Result;
use esp_idf_hal::io::Write;
use esp_idf_hal::modem::WifiModemPeripheral;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_svc::wifi::{self, AuthMethod, Configuration};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{
        server::{self, EspHttpServer},
        Method,
    },
    nvs::{EspDefaultNvsPartition, EspNvs, EspNvsPartition, NvsDefault},
    wifi::{BlockingWifi, EspWifi},
};
use heapless;
use log::*;
use std::thread::sleep;
use std::time::Duration;

struct WifiSettings {
    ssid: String,
    password: String,
}

/// Start an ap to configure the wifi.
/// Restarts when the wifi is configured.
fn wifi_wizard(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<WifiSettings> {
    // Get wifi configuration from NVS storage
    let nvs_default_partition: EspNvsPartition<NvsDefault> = EspDefaultNvsPartition::take()?;
    let mut nvs = EspNvs::new(nvs_default_partition, "wifi", true)?;

    // Read SSID and password from NVS
    let mut ssid = [0u8; 32];
    let mut password = [0u8; 64];

    let ssid_exists = match nvs.get_str("ssid", &mut ssid).unwrap() {
        Some(s) => {
            info!("Found stored SSID");
            true
        }
        None => {
            info!("No stored SSID found");
            false
        }
    };

    let password_exists = match nvs.get_str("password", &mut password).unwrap() {
        Some(p) => {
            info!("Found stored password");
            true
        }
        None => {
            info!("No stored password found");
            false
        }
    };

    if ssid_exists && password_exists {
        // Convert the raw bytes to strings, trimming null bytes
        let ssid_str = std::str::from_utf8(&ssid)?.trim_matches(char::from(0));
        let password_str = std::str::from_utf8(&password)?.trim_matches(char::from(0));
        info!("Found stored credentials - SSID: {}", ssid_str);
        return Ok(WifiSettings {
            ssid: ssid_str.to_string(),
            password: password_str.to_string(),
        });
    }

    info!("Starting wifi wizard");
    // Start AP mode
    let ap_config = Configuration::default();
    wifi.set_configuration(&ap_config)?;

    // Get the AP configuration
    let wifi_configuration: wifi::Configuration =
        wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
            ssid: heapless::String::try_from("E-Chess").expect("ssid too long"),
            auth_method: AuthMethod::None,
            ssid_hidden: false,
            max_connections: 4,

            ..Default::default()
        });

    // Set the configuration
    wifi.set_configuration(&wifi_configuration)?;

    // Start the wifi driver
    wifi.start()?;

    // Start AP
    info!("Starting access point...");
    wifi.()?;

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
            // Store credentials in NVS
            if let Err(e) = nvs.set_str("ssid", &settings.ssid) {
                error!("Failed to store SSID: {:?}", e);
            }
            if let Err(e) = nvs.set_str("password", &settings.password) {
                error!("Failed to store password: {:?}", e);
            }

            Ok(settings)
        }
        Err(_) => Err(anyhow::anyhow!("Failed to receive WiFi settings")),
    }
}

pub fn start_wifi_thread<M: WifiModemPeripheral>(modem: impl Peripheral<P = M>) -> Result<()> {
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(EspWifi::new(modem, sysloop.clone(), Some(nvs))?, sysloop)?;

    let config = wifi.get_configuration()?;
    info!("Wifi configuration: {:?}", config);

    let wifi_settings = wifi_wizard(&wifi)?;
    connect_wifi(&wifi, wifi_settings)?;

    info!("WIFI connection done");

    std::thread::spawn(move || -> anyhow::Result<()> {
        // Set the HTTP server
        let mut server = EspHttpServer::new(&server::Configuration::default()).unwrap();
        // http://<sta ip>/ handler
        server
            .fn_handler("/", Method::Get, |request| {
                let html = index_html();
                let mut response = request.into_ok_response()?;
                response.write_all(html.as_bytes())?;
                Ok(())
            })
            .unwrap();

        loop {
            sleep(Duration::from_millis(100));
        }
    });

    Ok(())
}

fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    wifi_settings: WifiSettings,
) -> anyhow::Result<()> {
    info!("Connect to SSID: {}", wifi_settings.ssid);
    sleep(Duration::from_secs(1));

    let wifi_configuration: wifi::Configuration =
        wifi::Configuration::Client(wifi::ClientConfiguration {
            ssid: heapless::String::try_from(wifi_settings.ssid.as_str()).expect("ssid too long"),
            bssid: None,
            auth_method: esp_idf_svc::wifi::AuthMethod::None,
            password: heapless::String::try_from(wifi_settings.password.as_str())
                .expect("password too long"),
            channel: None,
            ..Default::default()
        });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect().expect("could not connect");
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!(
        "IP: \n{}\n{}\n{:?}\n{:?}",
        ip_info.ip, ip_info.subnet, ip_info.dns, ip_info.secondary_dns
    );

    return Ok(());
}

fn index_html() -> String {
    format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>E-Chess</title>
        </head>
        <body>
           Hello World!
        </body>
        </html>
        "#
    )
}
