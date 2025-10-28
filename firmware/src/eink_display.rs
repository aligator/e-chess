use anyhow::Result;
use chess::BitBoard;
use chess_game::game::ChessGame;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;
use epd_waveshare::epd1in54_v2::Epd1in54;
use epd_waveshare::prelude::*;

pub struct ChessEinkDisplay<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    epd: Epd1in54<SPI, BUSY, DC, RST, DELAY>,
    spi: SPI,
    delay: DELAY,
}

impl<SPI, BUSY, DC, RST, DELAY> ChessEinkDisplay<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    pub fn new(
        mut spi: SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        mut delay: DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self> {
        let epd = Epd1in54::new(&mut spi, busy, dc, rst, &mut delay, delay_us).unwrap();
        Ok(Self { epd, spi, delay })
    }

    pub fn setup(&mut self) -> Result<()> {
        self.epd
            .clear_frame(&mut self.spi, &mut self.delay)
            .unwrap();
        self.epd
            .display_frame(&mut self.spi, &mut self.delay)
            .unwrap();

        // Set the display to sleep mode
        self.epd.sleep(&mut self.spi, &mut self.delay).unwrap();
        Ok(())
    }

    pub fn tick(&mut self, _physical: BitBoard, _game: &ChessGame) -> Result<()> {
        Ok(())
    }
}
// impl<'a> ChessEinkDisplay<'a> {
//     pub fn new(
//         epd: Epd1in54<
//             SpiDeviceDriver<'a, SpiDriver<'a>>,
//             PinDriver<'a, Gpio14>,
//             PinDriver<'a, Gpio13>,
//             PinDriver<'a, Gpio7>,
//             Ets,
//         >,
//     ) -> Self {
//         Self {
//             epd,
//             display: Display1in54::default(),
//             previous_state: None,
//         }
//     }

//     pub fn setup(&mut self) -> Result<()> {
//         // Clear the display
//         self.epd.clear_frame(&mut self.epd.spi(), &mut Ets)?;
//         self.epd.display_frame(&mut self.epd.spi(), &mut Ets)?;
//         Ok(())
//     }

//     fn get_pixel(square: Square) -> Point {
//         let rank = BOARD_SIZE - 1 - square.get_rank().to_index();
//         let file = square.get_file().to_index();

//         let x = file * 10; // 10 pixels per square
//         let y = rank * 10;

//         Point::new(x as i32, y as i32)
//     }

//     pub fn tick(&mut self, physical: BitBoard, game: &ChessGame) -> Result<()> {
//         let expected = game.expected_physical();

//         if self.previous_state != Some((physical, expected)) {
//             let diff = expected.diff(physical);

//             // Clear the display
//             self.display = Display1in54::default();

//             // Draw the board grid
//             for rank in 0..BOARD_SIZE {
//                 for file in 0..BOARD_SIZE {
//                     let style = if (rank + file) % 2 == 0 {
//                         PrimitiveStyle::with_fill(Color::White)
//                     } else {
//                         PrimitiveStyle::with_fill(Color::Black)
//                     };

//                     Rectangle::new(
//                         Point::new(file as i32 * 10, rank as i32 * 10),
//                         Size::new(10, 10),
//                     )
//                     .into_styled(style)
//                     .draw(&mut self.display)?;
//                 }
//             }

//             // Draw the pieces
//             diff.missing.for_each(|square| {
//                 let point = Self::get_pixel(square);
//                 Rectangle::new(point, Size::new(8, 8))
//                     .into_styled(PrimitiveStyle::with_fill(Color::Black))
//                     .draw(&mut self.display)
//                     .unwrap();
//             });

//             diff.added.for_each(|square| {
//                 let point = Self::get_pixel(square);
//                 Rectangle::new(point, Size::new(8, 8))
//                     .into_styled(PrimitiveStyle::with_fill(Color::White))
//                     .draw(&mut self.display)
//                     .unwrap();
//             });

//             // Update the display
//             self.epd
//                 .update_frame(&mut self.epd.spi(), &self.display.buffer(), &mut Ets)?;
//             self.epd.display_frame(&mut self.epd.spi(), &mut Ets)?;

//             self.previous_state = Some((physical, expected));
//         }

//         Ok(())
//     }
// }
