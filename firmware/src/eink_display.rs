use anyhow::Result;
use chess::BitBoard;
use chess_game::game::ChessGameState;
use debouncr::{debounce_2, Debouncer, Edge, Repeat2};
use embedded_graphics::mono_font::iso_8859_14::FONT_6X13;
use embedded_graphics::mono_font::iso_8859_4::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::prelude::{Point, Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use epd_waveshare::epd1in54::Display1in54;
use epd_waveshare::epd1in54_v2::Epd1in54;
use epd_waveshare::prelude::*;
use log::info;
use qrcode::QrCode;

use crate::event::EventManager;
use crate::wifi::{AccessPointInfo, ConnectionStateEvent, WifiInfo};
use crate::Event;

#[derive(Default)]
enum MenuState {
    #[default]
    ConnectionInfo,
    WebsiteQR,
    GameInfo,
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

    debouncer_a: Debouncer<u8, Repeat2>,
    debouncer_b: Debouncer<u8, Repeat2>,

    epd: Epd1in54<SPI, BUSY, DC, RST, DELAY>,
    spi: SPI,
    delay: DELAY,

    dirty: bool,

    display: Display1in54,
    small_text_style: MonoTextStyle<'static, Color>,
    normal_text_style: MonoTextStyle<'static, Color>,

    connection: Option<ConnectionStateEvent>,
    state: MenuState,

    event_rx: std::sync::mpsc::Receiver<Event>,
    event_tx: std::sync::mpsc::Sender<Event>,
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
        event_manager: &EventManager<Event>,
    ) -> Result<Self> {
        let epd = Epd1in54::new(&mut spi, busy, dc, rst, &mut delay, delay_us).unwrap();
        Ok(Self {
            button_a,
            button_b,
            debouncer_a: debounce_2(true),
            debouncer_b: debounce_2(true),

            epd,
            spi,
            delay,

            dirty: true,

            display: Display1in54::default(),
            small_text_style: MonoTextStyle::new(&FONT_4X6, Color::Black),
            normal_text_style: MonoTextStyle::new(&FONT_6X13, Color::Black),

            connection: Option::None,
            state: MenuState::default(),

            event_rx: event_manager.create_receiver(),
            event_tx: event_manager.create_sender(),
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        info!("Setup E-Paper display");

        self.epd.set_background_color(Color::White);

        // Clear the display
        self.clear_frame()?;
        self.update_and_display_frame()?;

        Ok(())
    }

    pub fn tick(&mut self, _physical: BitBoard, _game: &Option<ChessGameState>) -> Result<()> {
        // Get debounced button states
        let button_a = self
            .debouncer_a
            .update(
                self.button_a
                    .is_high()
                    .map_err(|err| anyhow::format_err!("could not read button a {:?}", err))?,
            )
            .is_some_and(|v| v == Edge::Rising);

        let button_b = self
            .debouncer_b
            .update(
                self.button_b
                    .is_high()
                    .map_err(|err| anyhow::format_err!("could not read button a {:?}", err))?,
            )
            .is_some_and(|v| v == Edge::Rising);

        if button_a {
            self.state = match self.state {
                MenuState::ConnectionInfo => MenuState::WebsiteQR,
                MenuState::WebsiteQR => MenuState::GameInfo,
                MenuState::GameInfo => MenuState::ConnectionInfo,
            };
            self.dirty = true;
        }

        match self.event_rx.try_recv() {
            Ok(event) => match event {
                Event::ConnectionState(connection_event) => {
                    self.connection = Some(connection_event);
                    self.dirty = true;
                }
                _ => {}
            },
            Err(_) => {}
        }

        if self.dirty {
            self.dirty = false;
            match self.state {
                MenuState::ConnectionInfo => {
                    if let Some(connection) = self.connection.clone() {
                        match connection {
                            ConnectionStateEvent::Wifi(wifi_info) => {
                                self.display_wifi_info(&wifi_info)?
                            }
                            ConnectionStateEvent::AccessPoint(access_point) => {
                                self.display_access_point_info(&access_point)?
                            }
                            ConnectionStateEvent::NotConnected => {
                                // Display not connected message
                                self.fill_empty()?;
                                Text::new(
                                    "Not connected",
                                    Point::new(10, 10),
                                    self.normal_text_style,
                                )
                                .draw(&mut self.display)?;
                                self.update_and_display_frame()?;
                            }
                        }
                    }
                }
                MenuState::WebsiteQR => {
                    self.display_website_info()?;
                }
                MenuState::GameInfo => {
                    self.fill_empty()?;
                    Text::new(
                        &format!("Game Info"),
                        Point::new(1, 1),
                        self.normal_text_style,
                    )
                    .draw(&mut self.display)?;

                    Text::new("TODO", Point::new(10, 10), self.normal_text_style)
                        .draw(&mut self.display)?;
                    self.update_and_display_frame()?;
                }
            }
        }

        Ok(())
    }

    fn clear_frame(&mut self) -> Result<()> {
        self.epd
            .clear_frame(&mut self.spi, &mut self.delay)
            .map_err(|err| anyhow::format_err!("could not clear the frame: {:?}", err))
    }

    fn update_and_display_frame(&mut self) -> Result<()> {
        self.epd
            .wake_up(&mut self.spi, &mut self.delay)
            .map_err(|err| anyhow::format_err!("could not wake up display: {:?}", err))?;
        self.epd
            .update_and_display_frame(&mut self.spi, self.display.buffer(), &mut self.delay)
            .map_err(|err| anyhow::format_err!("could not update and display frame: {:?}", err))?;
        self.epd
            .sleep(&mut self.spi, &mut self.delay)
            .map_err(|err| anyhow::format_err!("could not sleep display: {:?}", err))?;

        Ok(())
    }

    fn fill_empty(&mut self) -> Result<()> {
        Rectangle::new(
            Point::new(0, 0),
            Size::new(self.epd.width(), self.epd.height()),
        )
        .into_styled(PrimitiveStyle::with_fill(Color::White))
        .draw(&mut self.display)?;

        Ok(())
    }

    fn draw_qr_to_frame(&mut self, qr_content: &str, padding: u32) -> Result<(u32, u32, u32)> {
        // Generate QR code
        let qr = QrCode::new(qr_content.as_bytes())?;

        // Calculate QR code size and position to center it on the display
        let qr_size = qr.width() as u32;
        let display_width = self.epd.width();
        let display_height = self.epd.height();

        // Calculate scale to fit the screen exactly
        let scale = (display_width - padding * 2) / qr_size;
        let qr_scaled = qr_size * scale;
        let x_offset = (display_width - qr_scaled) / 2;
        let y_offset = (display_height - qr_scaled) / 2;

        let colors = qr.to_colors();
        for y in 0..qr_size {
            for x in 0..qr_size {
                if colors[y as usize * qr_size as usize + x as usize] == qrcode::Color::Dark {
                    let rect = Rectangle::new(
                        Point::new(
                            (x_offset + x as u32 * scale) as i32,
                            (y_offset + y as u32 * scale) as i32,
                        ),
                        Size::new(scale, scale),
                    )
                    .into_styled(PrimitiveStyle::with_fill(Color::Black));
                    rect.draw(&mut self.display)?;
                }
            }
        }

        Ok((x_offset, y_offset, qr_scaled))
    }

    fn display_website_info(&mut self) -> Result<()> {
        let ip = self
            .connection
            .clone()
            .map_or("N/A".to_string(), |conn| match conn {
                ConnectionStateEvent::Wifi(wifi_info) => wifi_info.ip.clone(),
                ConnectionStateEvent::AccessPoint(ap_info) => ap_info.ip.clone(),
                ConnectionStateEvent::NotConnected => "N/A".to_string(),
            });
        self.fill_empty()?;

        let padding: u32 = 14;
        let url = format!("http://{}", ip);
        let (x_offset, y_offset, qr_size) = self.draw_qr_to_frame(url.as_str(), padding)?;

        Text::new(
            &format!("Management UI"),
            Point::new(1, 1),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        Text::new(
            &format!("URL: {}", url),
            Point::new(x_offset as i32, (y_offset + qr_size + padding) as i32),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        self.update_and_display_frame()?;
        Ok(())
    }

    fn display_wifi_info(&mut self, wifi_info: &WifiInfo) -> Result<()> {
        self.fill_empty()?;

        Text::new(
            &format!("Wifi Connected"),
            Point::new(1, 1),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        Text::new(
            &format!("SSID: {}", wifi_info.ssid),
            Point::new(10, 10),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;
        Text::new(
            &format!("IP: {}", wifi_info.ip),
            Point::new(10, 30),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        self.update_and_display_frame()?;
        Ok(())
    }

    fn display_access_point_info(&mut self, access_point_info: &AccessPointInfo) -> Result<()> {
        self.fill_empty()?;

        // Create WiFi QR code content in the format: WIFI:S:<SSID>;T:WPA;P:<PASSWORD>;;
        let qr_content = format!(
            "WIFI:S:{};T:WPA;P:{};;",
            access_point_info.ssid, access_point_info.password
        );

        let padding: u32 = 14;
        let (x_offset, y_offset, qr_size) = self.draw_qr_to_frame(&qr_content, padding)?;

        Text::new(
            &format!("Access Point"),
            Point::new(1, 1),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        // Show ip address below the QR code
        Text::new(
            &format!("IP: {}", access_point_info.ip),
            Point::new(x_offset as i32, (y_offset + qr_size + padding) as i32),
            self.normal_text_style,
        )
        .draw(&mut self.display)?;

        // Update the display
        self.update_and_display_frame()?;

        Ok(())
    }
}
