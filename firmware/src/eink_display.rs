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
use embedded_graphics::text::renderer::TextRenderer;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use epd_waveshare::epd1in54::Display1in54;
use epd_waveshare::epd1in54_v2::Epd1in54;
use epd_waveshare::prelude::*;
use log::info;

use crate::event::EventManager;
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

    state: MenuState,

    event_rx: std::sync::mpsc::Receiver<Event>,
    event_tx: std::sync::mpsc::Sender<Event>,

    ble_pin: Option<u32>,
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

            state: MenuState::default(),

            event_rx: event_manager.create_receiver(),
            event_tx: event_manager.create_sender(),

            ble_pin: None
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
                Event::Setup(e) => match e {
                    crate::SetupEvent::BLEPin(ble_pin) => self.ble_pin = Some(ble_pin)
                }
                _ => {}
            },
            Err(_) => {}
        }

        if self.dirty {
            self.dirty = false;
            match self.state {
                _ => {
                    self.fill_empty()?;
                    Text::new(
                        &format!("BLE Pin"),
                        Point::new(1,  self.normal_text_style.line_height() as i32),
                        self.normal_text_style,
                    )
                    .draw(&mut self.display)?;

                    
                    if let Some(ble_pin) =  self.ble_pin {
                        Text::new(format!("{:0>6}", ble_pin).as_str(), Point::new(1, self.normal_text_style.line_height() as i32 * 2), self.normal_text_style)
                            .draw(&mut self.display)?;
                    } else {
                        Text::new("No BLE Pin", Point::new(10, 10), self.normal_text_style)
                            .draw(&mut self.display)?;
                    }
                    
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
}
