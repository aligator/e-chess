use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Ok, Result};
use board::Board;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::{self, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::ipv4::DHCPClientSettings;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ping::EspPing;
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::{io, ipv4};
use esp_idf_sys::{
    esp_netif_create_default_wifi_sta, esp_netif_get_ip_info, wifi_netif_driver,
    xTaskCreatePinnedToCore, xTaskGetCoreID, TaskHandle_t,
};
use log::*;
use std::thread::sleep;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;
mod board;

const WIFI_SSID: &str = "Freifunk";
const WIFI_PASS: &str = "";

fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    wifi_ssid: &str,
    wifi_password: &str,
) -> anyhow::Result<()> {
    let wifi_configuration: wifi::Configuration =
        wifi::Configuration::Client(wifi::ClientConfiguration {
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

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    info!(
        "IP: \n{}\n{}\n{:?}\n{:?}",
        ip_info.ip, ip_info.subnet, ip_info.dns, ip_info.secondary_dns
    );

    return Ok(());
}

struct AppParams<'a, const N: usize> {
    board: Board<'a, N>,
    led_pin: AnyIOPin,
    channel: esp_idf_hal::rmt::CHANNEL0, // For now only channel0 - don't know how to type this to support any channel...
}

const FIELD_SIZE: usize = 3;

static APP_PARAMS: Mutex<RefCell<Option<AppParams<FIELD_SIZE>>>> = Mutex::new(RefCell::new(None));

extern "C" fn app_loop_receiver(_: *mut c_void) {
    // Fetch the app params and remove it afterwards.
    let app_mu = APP_PARAMS.lock().unwrap();
    let app_mu_ref = app_mu.replace(None);
    drop(app_mu);

    let app = app_mu_ref.expect("app params not");
    let mut ws2812 = Ws2812Esp32Rmt::new(app.channel, app.led_pin).unwrap();
    let mut board = app.board;
    loop {
        board.tick();

        // for columns in board.field.iter() {
        //     info!("{:?}", columns);
        // }
        // info!("");

        // make black
        let mut pixels = [smart_leds::RGB { r: 0, g: 0, b: 0 }; 9];
        for (row, columns) in board.field.iter().enumerate() {
            for (column, value) in columns.iter().enumerate() {
                let mut pixel = row * board.size() + column;
                if row % 2 == 0 {
                    pixel = row * board.size() + (board.size() - column - 1);
                }

                if *value {
                    pixels[pixel] = smart_leds::RGB { r: 255, g: 0, b: 0 }
                } else {
                    pixels[pixel] = smart_leds::RGB { r: 0, g: 0, b: 0 }
                }
            }
        }

        ws2812.write_nocopy(pixels).unwrap();

        sleep(Duration::from_millis(100));
    }
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

/// Entry point to our application.
fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

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

    let mut board = Board::new(
        [
            AnyIOPin::from(peripherals.pins.gpio26),
            AnyIOPin::from(peripherals.pins.gpio27),
            AnyIOPin::from(peripherals.pins.gpio4),
        ],
        [
            AnyIOPin::from(peripherals.pins.gpio32),
            AnyIOPin::from(peripherals.pins.gpio33),
            AnyIOPin::from(peripherals.pins.gpio25),
        ],
    );
    board.setup();

    // To avoid interference with the wifi thread (on core0) all other app-logic is running on core 1.
    // Especially the LED strip may blink when wifi is used.
    // It doesn't seem to fix the problem fully, as with high wifi-load it still does flicker.
    // https://github.com/cat-in-136/ws2812-esp32-rmt-driver/issues/33
    let app_params = APP_PARAMS.lock().unwrap();
    app_params.replace(Some(AppParams {
        board: board,
        led_pin: AnyIOPin::from(peripherals.pins.gpio22),
        channel: peripherals.rmt.channel0,
    }));
    drop(app_params);

    unsafe {
        let name = CString::new("app-thread").unwrap();

        xTaskCreatePinnedToCore(
            Some(app_loop_receiver),
            name.as_ptr(),
            10000,
            std::ptr::null_mut(),
            15,
            std::ptr::null_mut(),
            1,
        );
    };

    // Set the HTTP server
    let mut server = EspHttpServer::new(&server::Configuration::default())?;
    // http://<sta ip>/ handler
    server.fn_handler("/", Method::Get, |request| {
        let html = index_html();
        let mut response = request.into_ok_response()?;
        response.write_all(html.as_bytes())?;
        Ok(())
    })?;

    loop {
        sleep(Duration::new(10, 0));
    }
}
