use anyhow::Result;
use chess::{BitBoard, Color, File, GameResult, Rank, Square};
use chess_game::game::{ChessGameState, PlayingState};
use smart_leds::RGB;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

use crate::constants::BOARD_SIZE;

// 28 border squares in clockwise order starting from a1.
// Square index = rank*8 + file  (rank 0 = rank1, file 0 = fileA)
const LOADING_SPINNER_SQUARES: [u8; 28] = [
    // bottom row a1→h1
    0, 1, 2, 3, 4, 5, 6, 7,
    // right col h2→h7
    15, 23, 31, 39, 47, 55,
    // top row h8→a8
    63, 62, 61, 60, 59, 58, 57, 56,
    // left col a7→a2
    48, 40, 32, 24, 16, 8,
];

const HEARTBEAT_TICKS: u32 = 80;

type Pixels = [RGB<u8>; BOARD_SIZE * BOARD_SIZE];

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
        DiffResult { _same: same, missing, added }
    }
}

fn blank() -> Pixels {
    [RGB { r: 0, g: 0, b: 0 }; BOARD_SIZE * BOARD_SIZE]
}

fn loser_color(result: Option<GameResult>) -> Option<Color> {
    result.and_then(|r| match r {
        GameResult::WhiteCheckmates | GameResult::BlackResigns => Some(Color::Black),
        GameResult::BlackCheckmates | GameResult::WhiteResigns => Some(Color::White),
        _ => None,
    })
}

fn opposite(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
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

    fn get_pixel(square: Square) -> usize {
        let rank = BOARD_SIZE - 1 - square.get_rank().to_index();
        let file = square.get_file().to_index();
        let mut pixel = rank * BOARD_SIZE + file;
        if rank % 2 == 0 {
            pixel = rank * BOARD_SIZE + (BOARD_SIZE - file - 1);
        }
        pixel
    }

    /// Scale r/g/b (0–255 range) by brightness factor.
    fn rgb(r: f32, g: f32, b: f32, brightness: f32) -> RGB<u8> {
        RGB {
            r: (r * brightness) as u8,
            g: (g * brightness) as u8,
            b: (b * brightness) as u8,
        }
    }

    /// King pulses red with frequency ramping up — panic effect.
    fn render_king_heartbeat(brightness: f32, elapsed: u32, king_sq: Square) -> Pixels {
        let t = elapsed as f32 / HEARTBEAT_TICKS as f32;
        let phase = std::f32::consts::TAU * (1.5 * t + 3.0 * t * t);
        let pulse = phase.sin().abs();
        let mut pixels = blank();
        pixels[Self::get_pixel(king_sq)] = Self::rgb(255.0 * pulse, 0.0, 0.0, brightness);
        pixels
    }

    /// Winner-colored rings expand outward from the losing king, looping.
    fn render_shockwave(
        brightness: f32,
        shock_elapsed: u32,
        king_sq: Square,
        winner_color: Color,
    ) -> Pixels {
        let ring_radius = (shock_elapsed % 55) as f32 * 0.22;
        let king_rank = king_sq.get_rank().to_index() as f32;
        let king_file = king_sq.get_file().to_index() as f32;

        let mut pixels = blank();
        for rank in 0..8usize {
            for file in 0..8usize {
                let sq = Square::make_square(Rank::from_index(rank), File::from_index(file));
                let dr = rank as f32 - king_rank;
                let df = file as f32 - king_file;
                let dist = (dr * dr + df * df).sqrt();
                let diff = (dist - ring_radius).abs();
                if diff < 1.5 {
                    let i = (1.0 - diff / 1.5) * 255.0;
                    pixels[Self::get_pixel(sq)] = match winner_color {
                        Color::White => Self::rgb(i, i * 0.63, 0.0, brightness),
                        Color::Black => Self::rgb(0.0, i * 0.31, i, brightness),
                    };
                }
            }
        }
        // King stays red through the shockwave.
        pixels[Self::get_pixel(king_sq)] = Self::rgb(255.0, 0.0, 0.0, brightness);
        pixels
    }

    /// Fading trail running around the board border.
    fn render_border_spinner(
        brightness: f32,
        tick: u32,
        speed_divisor: u32,
        r: f32,
        g: f32,
        b: f32,
    ) -> Pixels {
        const TRAIL: usize = 8;
        let head = (tick / speed_divisor) as usize % LOADING_SPINNER_SQUARES.len();
        let mut pixels = blank();
        for i in 0..TRAIL {
            let pos = (head + LOADING_SPINNER_SQUARES.len() - i) % LOADING_SPINNER_SQUARES.len();
            let fade = (TRAIL - i) as f32 / TRAIL as f32;
            pixels[Self::get_pixel(Square::new(LOADING_SPINNER_SQUARES[pos]))] =
                Self::rgb(r * fade, g * fade, b * fade, brightness);
        }
        pixels
    }

    /// Normal play: highlights for last move, moving piece, diff squares, and possible moves.
    fn render_board(brightness: f32, game: &ChessGameState) -> Pixels {
        let mut pixels = blank();

        if let Some(last_move) = game.last_move {
            pixels[Self::get_pixel(last_move.get_source())] = Self::rgb(0.0, 127.0, 127.0, brightness);
            pixels[Self::get_pixel(last_move.get_dest())] = Self::rgb(0.0, 255.0, 255.0, brightness);
        }

        if let PlayingState::MovingPiece { piece: _, from } = game.playing_state {
            // Source square is a valid placement target — highlight green.
            pixels[Self::get_pixel(from)] = Self::rgb(0.0, 255.0, 0.0, brightness);
        }

        let diff = game.expected_physical.diff(game.physical);
        diff.missing.for_each(|sq| {
            pixels[Self::get_pixel(sq)] = Self::rgb(255.0, 255.0, 0.0, brightness);
        });
        diff.added.for_each(|sq| {
            pixels[Self::get_pixel(sq)] = Self::rgb(255.0, 0.0, 0.0, brightness);
        });
        game.possible_moves.for_each(|sq| {
            pixels[Self::get_pixel(sq)] = Self::rgb(0.0, 255.0, 0.0, brightness);
        });

        pixels
    }

    pub fn tick(&mut self, game: &Option<ChessGameState>) -> Result<()> {
        if game.is_none() {
            return Ok(());
        }
        let game = game.unwrap();

        self.tick_counter = self.tick_counter.wrapping_add(1);
        let t = self.tick_counter;
        let b = self.brightness;

        let pixels = if let Some(loser) = loser_color(game.game_result) {
            let elapsed = t.wrapping_sub(*self.game_over_tick.get_or_insert(t));
            let king_sq = game.current_position.king_square(loser);
            self.previous_state = None;
            if elapsed < HEARTBEAT_TICKS {
                Self::render_king_heartbeat(b, elapsed, king_sq)
            } else {
                Self::render_shockwave(b, elapsed - HEARTBEAT_TICKS, king_sq, opposite(loser))
            }
        } else {
            self.game_over_tick = None;
            if game.is_loading {
                // White spinner — our move is being submitted.
                self.previous_state = None;
                Self::render_border_spinner(b, t, 4, 255.0, 255.0, 255.0)
            } else if game.opponent_is_thinking {
                // Blue spinner — opponent is thinking.
                self.previous_state = None;
                Self::render_border_spinner(b, t, 20, 0.0, 0.0, 255.0)
            } else {
                let state_key = (game.physical, game.expected_physical);
                if self.previous_state == Some(state_key) {
                    return Ok(());
                }
                self.previous_state = Some(state_key);
                Self::render_board(b, &game)
            }
        };

        self.leds.write_nocopy(pixels)?;
        Ok(())
    }
}
