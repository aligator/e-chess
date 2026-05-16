use anyhow::Result;
use chess::{BitBoard, Color, File, GameResult, Rank, Square};
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
    game_over_tick: Option<u32>,
}

impl<'a> Display<'a> {
    pub fn new(leds: Ws2812Esp32Rmt<'a>) -> Self {
        Self {
            leds,
            previous_state: None,
            brightness: 0.15,
            tick_counter: 0,
            game_over_tick: None,
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

        // Game-over animation: Phase C (king heartbeat) → Phase A (shockwave rings).
        let loser_color = game.game_result.and_then(|result| match result {
            GameResult::WhiteCheckmates | GameResult::BlackResigns => Some(Color::Black),
            GameResult::BlackCheckmates | GameResult::WhiteResigns => Some(Color::White),
            _ => None,
        });
        if let Some(loser_color) = loser_color {
            let winner_color = match loser_color {
                Color::White => Color::Black,
                Color::Black => Color::White,
            };
            let start = *self.game_over_tick.get_or_insert(self.tick_counter);
            let elapsed = self.tick_counter.wrapping_sub(start);

            let king_sq = game.current_position.king_square(loser_color);
            let king_rank = king_sq.get_rank().to_index() as f32;
            let king_file = king_sq.get_file().to_index() as f32;

            let mut pixels = [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE];

            const HEARTBEAT_TICKS: u32 = 80;
            if elapsed < HEARTBEAT_TICKS {
                // Frequency ramps from ~1.5 Hz to ~4.5 Hz then stops — panic pulse.
                let t = elapsed as f32 / HEARTBEAT_TICKS as f32;
                let phase = std::f32::consts::TAU * (1.5 * t + 3.0 * t * t);
                let brightness = phase.sin().abs();
                let r = (255.0 * self.brightness * brightness) as u8;
                pixels[Self::get_pixel(king_sq)] = RGB { r, g: 0, b: 0 };
            } else {
                // Shockwave rings expand from king, looping every 55 ticks.
                let shock_t = (elapsed - HEARTBEAT_TICKS) % 55;
                let ring_radius = shock_t as f32 * 0.22;

                for rank in 0..8usize {
                    for file in 0..8usize {
                        let sq = Square::make_square(
                            Rank::from_index(rank),
                            File::from_index(file),
                        );
                        let dist = {
                            let dr = rank as f32 - king_rank;
                            let df = file as f32 - king_file;
                            (dr * dr + df * df).sqrt()
                        };
                        let diff = (dist - ring_radius).abs();
                        if diff < 1.5 {
                            let intensity = (1.0 - diff / 1.5) * self.brightness;
                            let (r, g, b) = match winner_color {
                                Color::White => ((255.0 * intensity) as u8, (160.0 * intensity) as u8, 0),
                                Color::Black => (0, (80.0 * intensity) as u8, (255.0 * intensity) as u8),
                            };
                            pixels[Self::get_pixel(sq)] = RGB { r, g, b };
                        }
                    }
                }
                // King square stays red through the shockwave.
                let glow = (self.brightness * 255.0) as u8;
                pixels[Self::get_pixel(king_sq)] = RGB { r: glow, g: 0, b: 0 };
            }

            self.leds.write_nocopy(pixels)?;
            self.previous_state = None;
            return Ok(());
        }
        self.game_over_tick = None;

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
