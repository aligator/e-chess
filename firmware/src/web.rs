use anyhow::Result;
use chess::{BitBoard, File, Game, Rank, Square};
use esp_idf_hal::io::Write;
use esp_idf_svc::http::{server::EspHttpServer, Method};
use maud::{html, PreEscaped};
use std::sync::{Arc, Mutex};

use crate::wifi::page;

pub struct Web {
    game: Arc<Mutex<Option<chess::Game>>>,
}

unsafe fn handle_game(server: &mut EspHttpServer, game: Arc<Mutex<Option<Game>>>) -> Result<()> {
    server.fn_handler_nonstatic("/game", Method::Get, move |request| {
        let game = game.lock().unwrap();

        let html = if let Some(game) = &*game {
            let pieces = game.current_position();

            let mut table: String = String::new();

            for rank in (0..8).rev() {
                table += &format!("<tr><td>{}</td>", rank + 1);
                for file in 0..8 {
                    let square =
                        Square::make_square(Rank::from_index(rank), File::from_index(file));
                    let piece = pieces.piece_on(square);
                    let color = pieces.color_on(square);
                    let piece = match piece {
                        Some(chess::Piece::Pawn) => "♟",
                        Some(chess::Piece::Rook) => "♜",
                        Some(chess::Piece::Knight) => "♞",
                        Some(chess::Piece::Bishop) => "♝",
                        Some(chess::Piece::Queen) => "♛",
                        Some(chess::Piece::King) => "♚",
                        None => "",
                    };

                    let piece = match color {
                        Some(chess::Color::White) => {
                            format!("<span style='color: gray'>{}</span>", piece)
                        }
                        Some(chess::Color::Black) => {
                            format!("<span style='color: black'>{}</span>", piece)
                        }
                        None => piece.to_string(),
                    };

                    table += &format!("<td>{}</td>", piece);
                }
                table += "</tr>";
            }
            table += "<tr><td/><td>a</td><td>b</td><td>c</d><td>d</td><td>e</td><td>f</td><td>g</td><td>h</td></tr>";
            
            page(
                html!(
                    h1 { "E-Chess" }
                    p {"Current game state"}
                    table { (PreEscaped(table)) }
                    // small js to autoreload the page every 5 seconds
                    script { r#"
                        setTimeout(function() {
                            location.reload();
                        }, 5000);
                    "# }
                )
                .into_string(),
            )
        } else {
            page(
                html!(
                    h1 { "E-Chess" }
                    p {"No game state"}
                )
                .into_string(),
            )
        };
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    Ok(())
}

impl Web {
    pub fn new() -> Web {
        Web {
            game: Arc::new(Mutex::new(None)),
        }
    }

    pub fn register(&self, server: &mut EspHttpServer) -> Result<()> {
        println!("Registering Web");
        unsafe { handle_game(server, self.game.clone())? };
        Ok(())
    }

    pub fn tick(&mut self, game: chess::Game) {
        self.game.lock().unwrap().replace(game);
    }
}
