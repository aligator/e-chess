use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys::*;
use smart_leds::hsv::{hsv2rgb, Hsv};
use std::thread::sleep;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

/// Entry point to our application.
///
/// It sets up a Wi-Fi connection to the Access Point given in the
/// configuration, then blinks the RGB LED green/blue.
///
/// If the LED goes solid red, then it was unable to connect to your Wi-Fi
/// network.
fn main() -> Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let led_pin = peripherals.pins.gpio22;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, led_pin).unwrap();

    println!("Start NeoPixel rainbow!");

    let mut hue = unsafe { esp_random() } as u8;
    loop {
        let pixels = std::iter::repeat(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 8,
        }))
        .take(9);
        ws2812.write_nocopy(pixels).unwrap();

        sleep(Duration::from_millis(100));

        hue = hue.wrapping_add(10);
    }
}