use anyhow::Result;
use chess::BitBoard;
use chess_game::game::ChessGameState;
use embedded_graphics::prelude::*;
use embedded_graphics::prelude::{Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use epd_waveshare::epd1in54::Display1in54;
use epd_waveshare::epd1in54_v2::Epd1in54;
use epd_waveshare::prelude::*;
use log::info;
use qrcode::QrCode;

enum MenuState {
    GameInfo,
    HotspotQR,
    WebsiteQR,
}

pub struct ChessEinkDisplay<ButtonA, ButtonB, SPI, BUSY, DC, RST, DELAY>
where
    ButtonA: InputPin,
    ButtonB: InputPin,
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    button_a: ButtonA,
    button_b: ButtonB,
    epd: Epd1in54<SPI, BUSY, DC, RST, DELAY>,
    spi: SPI,
    delay: DELAY,
}

impl<ButtonA, ButtonB, SPI, BUSY, DC, RST, DELAY>
    ChessEinkDisplay<ButtonA, ButtonB, SPI, BUSY, DC, RST, DELAY>
where
    ButtonA: InputPin,
    ButtonB: InputPin,
    BUSY: InputPin,
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    pub fn new(
        button_a: ButtonA,
        button_b: ButtonB,
        mut spi: SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        mut delay: DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self> {
        let epd = Epd1in54::new(&mut spi, busy, dc, rst, &mut delay, delay_us).unwrap();
        Ok(Self {
            button_a,
            button_b,
            epd,
            spi,
            delay,
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        info!("Setup E-Paper display");

        // Clear the display
        self.epd
            .clear_frame(&mut self.spi, &mut self.delay)
            .unwrap();
        self.epd
            .display_frame(&mut self.spi, &mut self.delay)
            .unwrap();

        self.display_wifi_qr("E-Chess", "1337_e-chess")?;

        // Set the display to sleep mode
        self.epd.sleep(&mut self.spi, &mut self.delay).unwrap();
        Ok(())
    }

    pub fn tick(&mut self, _physical: BitBoard, _game: &ChessGameState) -> Result<()> {
        Ok(())
    }

    pub fn display_wifi_qr(&mut self, ssid: &str, password: &str) -> Result<()> {
        let mut display = Display1in54::default();

        // Fill the entire display with white
        let background = Rectangle::new(Point::new(0, 0), Size::new(200, 200))
            .into_styled(PrimitiveStyle::with_fill(Color::White));
        background.draw(&mut display)?;

        // Create WiFi QR code content in the format: WIFI:S:<SSID>;T:WPA;P:<PASSWORD>;;
        let qr_content = format!("WIFI:S:{};T:WPA;P:{};;", ssid, password);
        info!("Generating QR code for SSID: {}", ssid);

        // Generate QR code
        let qr = QrCode::new(qr_content.as_bytes())?;

        // Calculate QR code size and position to center it on the display
        let qr_size = qr.width() as u32;
        let display_width = 200; // E-ink display width
        let display_height = 200; // E-ink display height

        // Calculate scale to fit the screen exactly
        let scale = display_width / qr_size;
        let qr_width = qr_size * scale;
        let qr_height = qr_size * scale;
        let x_offset = (display_width - qr_width) / 2;
        let y_offset = (display_height - qr_height) / 2;

        let colors = qr.to_colors();
        for y in 0..qr_size {
            for x in 0..qr_size {
                if colors[y as usize * qr_size as usize + x as usize] == qrcode::Color::Dark {
                    info!(
                        "Drawing pixel at ({}, {}) size {}",
                        (x_offset + x as u32 * scale) as i32,
                        (x_offset + y as u32 * scale) as i32,
                        scale
                    );

                    let rect = Rectangle::new(
                        Point::new(
                            (x_offset + x as u32 * scale) as i32,
                            (y_offset + y as u32 * scale) as i32,
                        ),
                        Size::new(scale, scale),
                    )
                    .into_styled(PrimitiveStyle::with_fill(Color::Black));
                    rect.draw(&mut display)?;
                }
            }
        }

        // Update the display
        self.epd
            .update_and_display_frame(&mut self.spi, display.buffer(), &mut self.delay)
            .unwrap();

        Ok(())
    }
}
