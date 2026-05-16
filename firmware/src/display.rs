use anyhow::Result;
use chess::{BitBoard, Square};
use chess_game::game::ChessGameState;
use smart_leds::RGB;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::constants::BOARD_SIZE;

// 28 border squares in clockwise order starting from a1.
// Square index = rank*8 + file  (rank 0 = rank1, file 0 = fileA)
const BORDER_SQUARES: [u8; 28] = [
    // bottom row a1→h1
    0, 1, 2, 3, 4, 5, 6, 7,
    // right col h2→h7
    15, 23, 31, 39, 47, 55,
    // top row h8→a8
    63, 62, 61, 60, 59, 58, 57, 56,
    // left col a7→a2
    48, 40, 32, 24, 16, 8,
];

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
    brightness: f32,
    tick_counter: u32,
}

impl<'a> Display<'a> {
    pub fn new(leds: Ws2812Esp32Rmt<'a>) -> Self {
        Self {
            leds,
            previous_state: None,
            brightness: 0.15,
            tick_counter: 0,
        }
    }

    pub fn setup(&self) -> Result<()> {
        Ok(())
    }

    fn border_spinner(pixels: &mut [RGB<u8>; BOARD_SIZE * BOARD_SIZE], head: usize, r: u8, g: u8, b: u8) {
        const TRAIL: usize = 8;
        for i in 0..TRAIL {
            let pos = (head + BORDER_SQUARES.len() - i) % BORDER_SQUARES.len();
            let fade = (TRAIL - i) as f32 / TRAIL as f32;
            pixels[Self::get_pixel(Square::new(BORDER_SQUARES[pos]))] = RGB {
                r: (r as f32 * fade) as u8,
                g: (g as f32 * fade) as u8,
                b: (b as f32 * fade) as u8,
            };
        }
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

    pub fn tick(&mut self, game: &Option<ChessGameState>) -> Result<()> {
        if game.is_none() {
            return Ok(());
        }
        let game = game.unwrap();

        self.tick_counter = self.tick_counter.wrapping_add(1);

        if game.is_loading {
            // White trail running around board border — our move is being submitted.
            let idx = (self.tick_counter / 4) as usize % BORDER_SQUARES.len();
            let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];
            let b = (255.0 * self.brightness) as u8;
            Self::border_spinner(&mut pixels, idx, b, b, b);
            self.leds.write_nocopy(pixels)?;
            // Invalidate previous_state so normal rendering re-draws fully after loading.
            self.previous_state = None;
            return Ok(());
        }

        if game.opponent_is_thinking {
            // Blue trail running slowly around board border — opponent is thinking.
            let idx = (self.tick_counter / 20) as usize % BORDER_SQUARES.len();
            let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];
            let b = (255.0 * self.brightness) as u8;
            Self::border_spinner(&mut pixels, idx, 0, 0, b);
            self.leds.write_nocopy(pixels)?;
            self.previous_state = None;
            return Ok(());
        }

        if self.previous_state != Some((game.physical, game.expected_physical)) {
            let diff = game.expected_physical.diff(game.physical);
            let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];

            let last_move = game.last_move;

            // Colorize the last moved square.
            if let Some(last_move) = last_move {
                pixels[Self::get_pixel(last_move.get_source())] = RGB {
                    r: 0,
                    g: (127 as f32 * self.brightness) as u8,
                    b: (127 as f32 * self.brightness) as u8,
                };
                pixels[Self::get_pixel(last_move.get_dest())] = RGB {
                    r: 0,
                    g: (255 as f32 * self.brightness) as u8,
                    b: (255 as f32 * self.brightness) as u8,
                };
            };

            // Colorize the currently moving piece in blue
            if let chess_game::game::PlayingState::MovingPiece { piece: _, from } =
                game.playing_state
            {
                // Highlight the source square of the moving piece in green (as it is effectively a valid field for placement)
                pixels[Self::get_pixel(from)] = RGB {
                    r: 0,
                    g: (255 as f32 * self.brightness) as u8,
                    b: 0,
                };
            }

            diff.missing.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB {
                    r: (255 as f32 * self.brightness) as u8,
                    g: (255 as f32 * self.brightness) as u8,
                    b: 0,
                };
            });

            diff.added.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB {
                    r: (255 as f32 * self.brightness) as u8,
                    g: 0,
                    b: 0,
                };
            });

            game.possible_moves.for_each(|square| {
                pixels[Self::get_pixel(square)] = RGB {
                    r: 0,
                    g: (255 as f32 * self.brightness) as u8,
                    b: 0,
                };
            });

            self.leds.write_nocopy(pixels)?;
            self.previous_state = Some((game.physical, game.expected_physical));
        }

        Ok(())
    }
}
