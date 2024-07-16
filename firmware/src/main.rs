use std::time::Duration;

use anyhow::Result;
use esp_idf_hal::gpio::{AnyIOPin, Level, PinDriver, Pull};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::task::thread::ThreadSpawnConfiguration;

use std::thread::sleep;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

pub trait BoardPinExt {
    fn set_output(&mut self, high: bool);
    fn disable(&mut self);
}

pub struct Board<'a, const N: usize> {
    column_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Output>; N],
    row_pins: [PinDriver<'a, AnyIOPin, esp_idf_hal::gpio::Input>; N],

    pub field: [[bool; N]; N],
}

impl<'a, const N: usize> Board<'a, N> {
    pub fn size(&self) -> usize {
        N
    }

    fn new(
        column_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
        row_pins: [impl Peripheral<P = AnyIOPin> + 'a; N],
    ) -> Self {
        Board {
            column_pins: column_pins.map(|pin| PinDriver::output(pin).unwrap()),
            row_pins: row_pins.map(|pin| PinDriver::input(pin).unwrap()),

            field: [[false; N]; N],
        }
    }

    pub fn setup(&mut self) {
        // Set up the pullup.
        for pin in &mut self.row_pins {
            pin.set_pull(Pull::Up).unwrap();
        }

        // Set all columns high.
        for pin in &mut self.column_pins {
            pin.set_high().unwrap();
        }
    }

    pub fn tick(&mut self) {
        // Check each field
        for (col, col_pin) in &mut self.column_pins.iter_mut().enumerate() {
            col_pin.set_low().unwrap();

            for (row, row_pin) in &mut self.row_pins.iter().enumerate() {
                self.field[row][col] = row_pin.get_level() == Level::Low;
            }

            col_pin.set_high().unwrap();
        }
    }
}

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

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    ThreadSpawnConfiguration {
        name: Some(b"app-thread\0"),
        stack_size: 10000,
        priority: 15,
        ..Default::default()
    }
    .set()
    .unwrap();

    let peripherals = Peripherals::take().unwrap();
    let led_pin = peripherals.pins.gpio22;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, led_pin).unwrap();

    let _thread_1 = std::thread::Builder::new()
        .spawn(move || {
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

            loop {
                // make black
                let mut pixels = [smart_leds::RGB { r: 0, g: 0, b: 0 }; 9];

                board.tick();

                // for columns in board.field.iter() {
                //     info!("{:?}", columns);
                // }
                // info!("");

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
        })
        .unwrap();

    _thread_1.join().unwrap();
    loop {}
}
