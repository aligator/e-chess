use anyhow::{Ok, Result};
use esp_idf_hal::{io::Write, modem};
use esp_idf_svc::wifi;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{
        server::{self, EspHttpServer},
        Method,
    },
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use log::*;
use std::thread::sleep;
use std::{cell::RefCell, os::raw::c_void, sync::Mutex, time::Duration};

pub(crate) struct WifiParams {
    pub(crate) modem: modem::Modem,
    pub(crate) static WIFI_PARAMS: Mutex<RefCell<Option<WifiParams>>> = Mutex::new(RefCell::new(None));
}

const WIFI_SSID: &str = "Freifunk";
const WIFI_PASS: &str = "";

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    info!("Connect to SSID: {}", WIFI_SSID);
    sleep(Duration::from_secs(1));

    let wifi_configuration: wifi::Configuration =
        wifi::Configuration::Client(wifi::ClientConfiguration {
            ssid: WIFI_SSID.try_into().expect("ssid could not be read"),
            bssid: None,
            auth_method: esp_idf_svc::wifi::AuthMethod::None,
            password: WIFI_PASS.try_into().expect("password could not be read"),
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

pub(crate) extern "C" fn wifi_loop_receiver(_: *mut c_void) {
    wifi_loop().unwrap()
}

fn wifi_loop() -> Result<()> {
    // Fetch the wifi params and remove it afterwards.
    let wifi_mu = WIFI_PARAMS.lock().unwrap();
    let wifi_mu_ref = wifi_mu.replace(None);
    drop(wifi_mu);

    let wifi_param = wifi_mu_ref.expect("wifi params not");

    let sysloop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(wifi_param.modem, sysloop.clone(), Some(nvs)).unwrap(),
        sysloop,
    )
    .unwrap();

    connect_wifi(&mut wifi).unwrap();

    info!("WIFI connection done");

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
}
