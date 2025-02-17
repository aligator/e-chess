use anyhow::{Ok, Result};
use chess::BitBoard;
use chess_game::{bitboard_extensions::*, game::ChessGame};
use smart_leds::RGB;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::constants::BOARD_SIZE;

struct DiffResult {
    same: BitBoard,
    missing: BitBoard,
    added: BitBoard,
}

trait BitBoardDiff {
    fn diff(&self, other: BitBoard) -> DiffResult;
}

impl BitBoardDiff for BitBoard {
    fn diff(&self, other: BitBoard) -> DiffResult {
        let same = self & other;
        let missing = !self & other;
        let added = self & !other;
        DiffResult {
            same,
            missing,
            added,
        }
    }
}

pub struct Display<'a> {
    leds: Ws2812Esp32Rmt<'a>,
}

impl<'a> Display<'a> {
    pub fn new(leds: Ws2812Esp32Rmt<'a>) -> Self {
        Self { leds }
    }

    pub fn setup(&self) -> Result<()> {
        Ok(())
    }

    pub fn tick(&mut self, physical: BitBoard, game: &ChessGame) -> Result<()> {
        let diff = game.expected_physical().diff(physical);
        let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];

        physical.for_each(|square| {
            let rank = square.get_rank().to_index();
            let file = square.get_file().to_index();

            let mut pixel = rank * BOARD_SIZE + file;
            if rank % 2 == 0 {
                pixel = rank * BOARD_SIZE + (BOARD_SIZE - file - 1);
            }

            // If the square is missing colorize it yellow
            if diff.missing.get(square) == 1 {
                pixels[pixel] = RGB { r: 10, g: 10, b: 0 };
                return;
            } else if diff.added.get(square) == 1 {
                // If the square is added colorize it red
                pixels[pixel] = RGB { r: 10, g: 0, b: 0 };
                return;
            }
        });

        self.leds.write_nocopy(pixels)?;

        Ok(())
    }
}
