use crate::board::Board;
#[cfg(feature = "no_board")]
use crate::board::FakeBoard;
use crate::event::EventManager;
use crate::game::GameCommandEvent;
use crate::wifi::ConnectionStateEvent;
use anyhow::Result;
use board::MCP23017Board;
use chess::BitBoard;
use chess_game::game::ChessGameState;
use eink_display::ChessEinkDisplay;
use embedded_hal::spi::SpiDevice;
use esp_idf_hal::gpio::{Gpio0, PinDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::rmt::TxRmtDriver;
use esp_idf_hal::spi::config::DriverConfig;
use esp_idf_hal::spi::{SpiConfig, SpiDeviceDriver, SpiDriver};
use esp_idf_hal::{i2c::*, rmt::config::TransmitConfig};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use game::GameStateEvent;
use log::*;
use std::time::Duration;
use std::{thread, thread::sleep};
use storage::Storage;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;
mod eink_display;
mod event;
mod game;
mod request;
mod storage;
mod web;
mod wifi;

#[derive(Debug, Clone)]
enum Event {
    GameState(GameStateEvent),
    GameCommand(GameCommandEvent),
    ConnectionState(ConnectionStateEvent),
}

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

fn run_game<'a, ButtonA, ButtonB, SPI, BUSY, DC, RST, DELAY>(
    token: Option<String>,
    mcp23017: I2cDriver<'_>,
    ws2812: Ws2812Esp32Rmt,
    eink_display: &mut ChessEinkDisplay<ButtonA, ButtonB, SPI, BUSY, DC, RST, DELAY>,
    mut server: &mut EspHttpServer<'static>,
    event_manager: &EventManager<Event>,
) -> Result<()>
where
    ButtonA: embedded_hal::digital::InputPin,
    ButtonB: embedded_hal::digital::InputPin,
    SPI: SpiDevice,
    BUSY: embedded_hal::digital::InputPin,
    DC: embedded_hal::digital::OutputPin,
    RST: embedded_hal::digital::OutputPin,
    DELAY: embedded_hal::delay::DelayNs + 'a,
{
    #[cfg(not(feature = "no_board"))]
    let mut board: Box<dyn Board> = Box::from(MCP23017Board::new(mcp23017, 0x20));
    #[cfg(feature = "no_board")]
    let mut board: Box<dyn Board> = Box::from(FakeBoard {});

    board.setup()?;

    let mut display = display::Display::new(ws2812);
    display.setup()?;

    let test_rx = event_manager.create_receiver();
    thread::spawn(move || {
        while let Ok(event) = test_rx.recv() {
            info!("Received event: {:?}", event);
        }
    });

    let settings = game::Settings {
        token: token.unwrap_or_default(),
    };

    let web = web::Web::new();
    web.register(&mut server, &event_manager)?;
    info!("Registered web interface");

    game::run_game(settings, &event_manager);

    // Start the main loop
    info!("Start app loop");

    let rx = event_manager.create_receiver();
    let tx = event_manager.create_sender();
    let mut last_physical = BitBoard::new(0);
    let mut last_game_state: Option<ChessGameState> = None;

    tx.send(Event::GameCommand(GameCommandEvent::LoadNewGame(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
    )))?;

    // Start the event manager after setting up everything.
    event_manager.start_thread();
    loop {
        // Tick the physical board
        let physical = board.tick(last_physical)?;
        if physical != last_physical {
            last_physical = physical;
            if let Err(e) = tx.send(Event::GameCommand(GameCommandEvent::UpdatePhysical(
                physical,
            ))) {
                warn!("Failed to send update physical event: {:?}", e);
            }
        }

        // Check if event happens
        if let Ok(event) = rx.try_recv() {
            match event {
                Event::GameState(game_state_event) => match game_state_event {
                    GameStateEvent::UpdateGame(game_state) => {
                        info!("Received update game state event: {:?}", game_state);

                        last_game_state = Some(game_state);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(game_state) = last_game_state.clone() {
            display.tick(&game_state)?;
            eink_display.tick(physical, &game_state)?;
        }

        // Sleep to reduce CPU usage
        sleep(Duration::from_millis(100));
    }
}

/// Entry point to our application.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    info!("Starting E-Chess!");
    #[cfg(esp32)]
    info!("Running on ESP32");
    #[cfg(esp32s3)]
    info!("Running on ESP32S3");

    // Initialize the I2C bus for the MCP23017
    let config = I2cConfig::new().baudrate(100.kHz().into());
    #[cfg(esp32)]
    let mcp23017: I2cDriver<'_> = {
        let sda = peripherals.pins.gpio21;
        let scl = peripherals.pins.gpio22;
        I2cDriver::new(peripherals.i2c0, sda, scl, &config)?
    };
    #[cfg(esp32s3)]
    let mcp23017: I2cDriver<'_> = {
        let sda = peripherals.pins.gpio8;
        let scl = peripherals.pins.gpio9;
        I2cDriver::new(peripherals.i2c0, sda, scl, &config)?
    };

    let driver_config = TransmitConfig::new()
        .clock_divider(1) // Required parameter.
        .mem_block_num(8); // Increase the number depending on your code.

    // Initialize the RMT driver for the WS2812
    #[cfg(esp32)]
    let driver = TxRmtDriver::new(
        peripherals.rmt.channel0,
        peripherals.pins.gpio23,
        &driver_config,
    )?;
    #[cfg(esp32s3)]
    let driver = TxRmtDriver::new(
        peripherals.rmt.channel0,
        peripherals.pins.gpio4,
        &driver_config,
    )?;
    let ws2812 = Ws2812Esp32Rmt::new_with_rmt_driver(driver)?;

    // Initialize the SPI bus for the E-Paper display
    let spi_config = SpiConfig::new()
        .baudrate(Hertz(4_000_000))
        .data_mode(esp_idf_hal::spi::config::MODE_0);
    let driver_config = DriverConfig::new();
    // Create delay provider
    let delay = esp_idf_hal::delay::Ets;
    #[cfg(esp32)]
    let (spi_driver, cs, busy, dc, rst, button_a, button_b) = {
        let spi = peripherals.spi2;
        let sclk = peripherals.pins.gpio18;
        let mosi = peripherals.pins.gpio23;
        let cs = peripherals.pins.gpio5;
        let dc = peripherals.pins.gpio2;
        let busy = peripherals.pins.gpio4;
        let rst = peripherals.pins.gpio0;

        let spi_driver = SpiDriver::new(spi, sclk, mosi, Option::<Gpio0>::None, &driver_config)?;

        let button_a = peripherals.pins.gpio35;
        let button_b = peripherals.pins.gpio36;

        (spi_driver, cs, busy, dc, rst, button_a, button_b)
    };

    #[cfg(esp32s3)]
    let (spi_driver, cs, busy, dc, rst, button_a, button_b) = {
        let spi = peripherals.spi2;
        let sclk = peripherals.pins.gpio13;
        let mosi = peripherals.pins.gpio14;
        let cs = peripherals.pins.gpio2;
        let dc = peripherals.pins.gpio3;
        let busy = peripherals.pins.gpio5;
        let rst = peripherals.pins.gpio11;

        let spi_driver = SpiDriver::new(spi, sclk, mosi, Option::<Gpio0>::None, &driver_config)?;

        let button_a = peripherals.pins.gpio35;
        let button_b = peripherals.pins.gpio36;

        (spi_driver, cs, busy, dc, rst, button_a, button_b)
    };
    let spi = SpiDeviceDriver::new(spi_driver, Some(cs), &spi_config)?;

    // Setup the button pins and enable the internal pullup.
    let mut button_a_driver = PinDriver::input(button_a)?;
    button_a_driver.set_pull(esp_idf_hal::gpio::Pull::Up)?;
    let mut button_b_driver = PinDriver::input(button_b)?;
    button_b_driver.set_pull(esp_idf_hal::gpio::Pull::Up)?;

    let event_manager = EventManager::<Event>::new();

    let mut eink_display = eink_display::ChessEinkDisplay::new(
        button_a_driver,
        button_b_driver,
        spi,
        PinDriver::input(busy)?,
        PinDriver::output(dc)?,
        PinDriver::output(rst)?,
        delay,
        None,
        &event_manager,
    )?;

    let nvs = EspDefaultNvsPartition::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs.clone()))?;
    let storage = Storage::new(nvs.clone())?;

    let token = storage.get_str::<25>("api_token")?;
    info!("API Token: {:?}", token);

    eink_display.setup()?;

    let mut server = wifi::start_wifi(&event_manager, wifi_driver, storage)?;

    match run_game(
        token,
        mcp23017,
        ws2812,
        &mut eink_display,
        &mut server,
        &event_manager,
    ) {
        Ok(_) => {
            warn!("Stopping game loop");
            Ok(())
        }
        Err(e) => {
            warn!("Stopping game loop due to error: {:?}", e);
            loop {
                sleep(Duration::from_millis(1000));
            }
        }
    }
}
