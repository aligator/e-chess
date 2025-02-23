use std::str::FromStr;

use anyhow::{Ok, Result};
use chess::{BitBoard, Square};
use chess_game::{
    bitboard_extensions::*,
    game::{ChessGame, ChessState},
};
use log::*;
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
        let missing = self & !other;
        let added = !self & other;
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

    fn get_pixel(square: Square) -> usize {
        let rank = BOARD_SIZE - 1 - square.get_rank().to_index();
        let file = square.get_file().to_index();

        let mut pixel = rank * BOARD_SIZE + file;
        if rank % 2 == 0 {
            pixel = rank * BOARD_SIZE + (BOARD_SIZE - file - 1);
        }

        pixel
    }

    pub fn tick(&mut self, physical: BitBoard, game: &ChessGame) -> Result<()> {
        let diff = game.expected_physical().diff(physical);
        let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];

        diff.missing.for_each(|square| {
            pixels[Self::get_pixel(square)] = RGB { r: 20, g: 20, b: 0 };
        });

        diff.added.for_each(|square| {
            pixels[Self::get_pixel(square)] = RGB { r: 20, g: 0, b: 0 };
        });

        game.get_possible_moves().for_each(|square| {
            pixels[Self::get_pixel(square)] = RGB { r: 0, g: 20, b: 0 };
        });

        self.leds.write_nocopy(pixels)?;

        Ok(())
    }
}
