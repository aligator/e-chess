use anyhow::Result;
use esp_idf_hal::delay::BLOCK;
use esp_idf_hal::i2c::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use log::*;
use std::thread::sleep;
use std::time::Duration;

/// Entry point to our application.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    info!("Starting io expander mcp23017 test!");

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mut mcp23017 = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    info!("configured i2c");
    sleep(Duration::from_millis(1000));

    info!("Start configuring mcp23017");
    let addr: u8 = 0x20;
    // Configure GPA = input
    // Configure GPB = output
    let msg = &[0x00, 0xFF, 0x00];
    match mcp23017.write(addr, msg, BLOCK) {
        Ok(_) => info!("Successfully configured input/output ports"),
        Err(e) => {
            error!("Failed to configure I/O directions: {:?}", e);
            return Err(e.into());
        }
    };

    // Enable Pull ups for the inputs
    let pullup_msg = &[0x0C, 0xFF]; // 0x0D is GPPUB register, 0xFF enables pull-ups for all pins
    match mcp23017.write(addr, pullup_msg, BLOCK) {
        Ok(_) => info!("Successfully enabled pullups"),
        Err(e) => {
            error!("Failed to configure pullups: {:?}", e);
            return Err(e.into());
        }
    };

    loop {
        info!("Looping");

        // Set register pointer to GPIOA (0x12)
        mcp23017.write(addr, &[0x12], BLOCK)?;

        // Read from Port A (inputs)
        let mut input_data = [0u8; 1];
        mcp23017.read(addr, &mut input_data, BLOCK)?;

        // Invert the input data
        let inverted_data = !input_data[0];
        info!("Inverted data: 0x{:02X}", inverted_data);

        // Write the inverted input value to Port B (outputs)
        let write_msg = &[0x13, inverted_data]; // 0x13 is GPIOB register
        mcp23017.write(addr, write_msg, BLOCK)?;

        sleep(Duration::from_millis(100));
    }
}
