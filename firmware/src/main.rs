use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use board::Board;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys::xTaskCreatePinnedToCore;
use log::*;
use std::thread::sleep;
use wifi::{wifi_loop_receiver, WifiParams, WIFI_PARAMS};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;
mod board;
mod wifi;

const FIELD_SIZE: usize = 3;

struct AppParams<'a, const N: usize> {
    board: Board<'a, N>,
    led_pin: AnyIOPin,
    channel: esp_idf_hal::rmt::CHANNEL0, // For now only channel0 - don't know how to type this to support any channel...
}
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

/// Entry point to our application.
fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

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
    //
    // Note that I did not get it working to pass the parameters as pvParameters.
    // So I now pre-fill a mutex which is read inside the thread.
    let app_params = APP_PARAMS.lock().unwrap();
    app_params.replace(Some(AppParams {
        board: board,
        led_pin: AnyIOPin::from(peripherals.pins.gpio23),
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
            24,
            std::ptr::null_mut(),
            1,
        );
    };

    let wifi_params = WIFI_PARAMS.lock().unwrap();
    wifi_params.replace(Some(WifiParams {
        modem: peripherals.modem,
    }));
    drop(wifi_params);

    unsafe {
        let name = CString::new("wifi-thread").unwrap();
        xTaskCreatePinnedToCore(
            Some(wifi_loop_receiver),
            name.as_ptr(),
            10000,
            std::ptr::null_mut(),
            1,
            std::ptr::null_mut(),
            0,
        );
    };

    loop {
        sleep(Duration::new(10, 0));
    }
}
