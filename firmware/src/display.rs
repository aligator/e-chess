use anyhow::Result;
use chess::{BitBoard, Square};
use chess_game::game::ChessGameState;
use smart_leds::RGB;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::constants::BOARD_SIZE;

struct DiffResult {
    _same: BitBoard,
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
            _same: same,
            missing,
            added,
        }
    }
}

pub struct Display<'a> {
    leds: Ws2812Esp32Rmt<'a>,
    previous_state: Option<(BitBoard, BitBoard)>,
}

impl<'a> Display<'a> {
    pub fn new(leds: Ws2812Esp32Rmt<'a>) -> Self {
        Self {
            leds,
            previous_state: None,
        }
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

    pub fn tick(&mut self, physical: BitBoard, game: &ChessGameState) -> Result<()> {
        let expected = game.physical;

        if self.previous_state != Some((physical, expected)) {
            let diff = expected.diff(physical);
            let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];

            let last_move = game.last_move;

            // Colorize the last moved square.
            if let Some(last_move) = last_move {
                pixels[Self::get_pixel(last_move.get_source())] = RGB { r: 0, g: 5, b: 5 };
                pixels[Self::get_pixel(last_move.get_dest())] = RGB { r: 0, g: 20, b: 20 };
            };

            // Colorize the currently moving piece in blue
            if let chess_game::game::PlayingState::MovingPiece { piece: _, from } =
                game.playing_state
            {
                // Highlight the source square of the moving piece in green (as it is effectively a valid field for placement)
                pixels[Self::get_pixel(from)] = RGB { r: 0, g: 20, b: 0 };
            }

            diff.missing.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB { r: 20, g: 20, b: 0 };
            });

            diff.added.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB { r: 20, g: 0, b: 0 };
            });

            game.possible_moves.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB { r: 0, g: 20, b: 0 };
            });

            self.leds.write_nocopy(pixels)?;
            self.previous_state = Some((physical, expected));
        }

        Ok(())
    }
}
