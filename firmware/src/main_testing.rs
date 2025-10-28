#![deny(warnings)]

use anyhow::Result;
use embedded_hal::{delay::DelayNs, spi::MODE_0};
use epd_waveshare::{epd1in54_v2::Epd1in54, prelude::*};
use esp_idf_hal::{
    gpio::{Gpio11, PinDriver},
    peripherals::Peripherals,
    prelude::*,
    spi::{config::DriverConfig, Dma, SpiConfig, SpiDeviceDriver, SpiDriver},
};
use esp_idf_svc::log::EspLogger;
use log::info;

// activate spi, gpio in raspi-config
// needs to be run with sudo because of some sysfs_gpio permission problems and follow-up timing problems
// see https://github.com/rust-embedded/rust-sysfs-gpio/issues/5 and follow-up issues

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("Starting E-Paper display test");

    let peripherals = Peripherals::take().unwrap();
    info!("Peripherals initialized");

    // Configure SPI
    let spi_config = SpiConfig::new().baudrate(2.MHz().into()).data_mode(MODE_0);
    let driver_config = DriverConfig {
        dma: Dma::Disabled,
        ..Default::default()
    };
    info!("SPI configuration created");

    // Create delay provider
    let mut delay = esp_idf_hal::delay::Ets;

    // Configure pins for ESP32-S3
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio13;
    let mosi = peripherals.pins.gpio14;
    let cs = peripherals.pins.gpio2;
    let dc = peripherals.pins.gpio3;
    let busy = peripherals.pins.gpio5;
    let rst = peripherals.pins.gpio4;
    info!("Pins configured");

    let spi_driver = SpiDriver::new(spi, sclk, mosi, Option::<Gpio11>::None, &driver_config)?;
    let mut spi = SpiDeviceDriver::new(spi_driver, Some(cs), &spi_config)?;
    info!("SPI driver initialized");

    // Setup the EPD
    info!("Initializing E-Paper display...");
    let mut epd = Epd1in54::new(
        &mut spi,
        PinDriver::input(busy)?,
        PinDriver::output(dc)?,
        PinDriver::output(rst)?,
        &mut delay,
        None,
    )?;
    info!("E-Paper display initialized successfully");

    // Clear the full screen
    info!("Clearing screen...");
    epd.clear_frame(&mut spi, &mut delay)?;
    epd.display_frame(&mut spi, &mut delay)?;
    info!("Screen cleared");

    // Speeddemo
    info!("Starting speed demo...");
    epd.set_lut(&mut spi, &mut delay, Some(RefreshLut::Quick))?;
    let small_buffer = [Color::Black.get_byte_value(); 32]; //16x16
    let number_of_runs = 1;
    for i in 0..number_of_runs {
        let offset = i * 8 % 150;
        info!("Drawing partial frame at offset {}", offset);
        epd.update_partial_frame(
            &mut spi,
            &mut delay,
            &small_buffer,
            25 + offset,
            25 + offset,
            16,
            16,
        )?;
        epd.display_frame(&mut spi, &mut delay)?;
    }
    info!("Speed demo completed");

    // Clear the full screen
    info!("Clearing screen again...");
    epd.clear_frame(&mut spi, &mut delay)?;
    epd.display_frame(&mut spi, &mut delay)?;
    info!("Screen cleared");

    // Draw some squares
    info!("Drawing squares...");
    let small_buffer = [Color::Black.get_byte_value(); 3200]; //160x160
    epd.update_partial_frame(&mut spi, &mut delay, &small_buffer, 20, 20, 160, 160)?;

    let small_buffer = [Color::White.get_byte_value(); 800]; //80x80
    epd.update_partial_frame(&mut spi, &mut delay, &small_buffer, 60, 60, 80, 80)?;

    let small_buffer = [Color::Black.get_byte_value(); 8]; //8x8
    epd.update_partial_frame(&mut spi, &mut delay, &small_buffer, 96, 96, 8, 8)?;

    // Display updated frame
    info!("Displaying final frame...");
    epd.display_frame(&mut spi, &mut delay)?;
    info!("Final frame displayed");
    delay.delay_ms(5000);

    // Set the EPD to sleep
    info!("Putting display to sleep...");
    epd.sleep(&mut spi, &mut delay)?;
    info!("Display is now in sleep mode");

    Ok(())
}
