use anyhow::Result;
use chess::{BoardStatus, File, Rank, Square};
use chess_game::game::ChessGameState;
use esp_idf_hal::io::Write;
use esp_idf_svc::http::{server::EspHttpServer, Method};
use maud::html;
use serde_json::json;
use core::panic;
use std::{sync::{mpsc, Arc, Mutex}, thread};

use crate::{event::EventManager, game::{GameCommandEvent, GameStateEvent}, wifi::page, Event};


pub struct Web {
    game: Arc<Mutex<Option<ChessGameState>>>,
    game_key: Arc<Mutex<String>>,
}

unsafe fn handle_js(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic("/board.js", Method::Get, move |request| -> Result<()> {
        // Include the JavaScript file at compile time
        const JS: &[u8] = include_bytes!("../assets/board.js");

        let mut response = request.into_response(200, None, &[
            ("Content-Type", "application/javascript"),
        ])?;
        response.write_all(JS)?;
        Ok(())
    })?;
    Ok(())
}

unsafe fn handle_game(server: &mut EspHttpServer, current_game_key: Arc<Mutex<String>>) -> Result<()> {
    server.fn_handler_nonstatic("/game", Method::Get, move |request| {
        let current_game_key = current_game_key.lock().unwrap().clone();

        // Always use the same page structure
        let html = page(
            html!(
                // Game ID control - always visible
                div class="game-selector" {
                    label for="gameKey" { "Game ID or FEN: " }
                    input type="text" id="gameKey" value=(current_game_key) {}
                    button id="loadGame" { "Load Game" }
                }
               
                // Game info section - will be shown/hidden via JS
                div id="game-info" class=("game-info hidden") {
                    div class="status-container" {
                        p id="game-status" class="status" { "" }
                    }
                    p id="active-player" class="active-player" { 
                        ""
                    }
                }
                
                // Board container - will be populated via AJAX
                div id="board-container" {}
                
                script src="/board.js" {}
            )
            .into_string(),
        );
        
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    Ok(())
}

unsafe fn handle_load_game(server: &mut EspHttpServer, sender: mpsc::Sender<Event>, game_id: Arc<Mutex<String>>) -> Result<()> {
    server.fn_handler_nonstatic("/load-game", Method::Get, move |request| -> Result<()> {
        let uri = request.uri();
        
        // Parse the query string to get the game ID
        if let Some(query) = uri.split('?').nth(1) {
            if let Some(id_param) = query.split('&').find(|p| p.starts_with("key=")) {
                if let Some(id) = id_param.split('=').nth(1) {
                    // decode the id
                    let id = urlencoding::decode(id)?;
                    println!("Loading game: {}", id);

                    *game_id.lock().unwrap() = "".to_string();
                    
                    // Trigger event to load new game
                    sender.send(Event::GameCommand(GameCommandEvent::LoadNewGame(id.to_string()))).unwrap();
                }
            }
        }
        
        let mut response = request.into_ok_response()?;
        response.write_all(b"OK")?;
        Ok(())
    })?;
    
    Ok(())
}

// Send game data to the client
unsafe fn handle_game_data(server: &mut EspHttpServer, game: Arc<Mutex<Option<ChessGameState>>>, game_id: Arc<Mutex<String>>) -> Result<()> {
    server.fn_handler_nonstatic("/game-data", Method::Get, move |request| -> Result<()> {
        let game = game.lock().unwrap();
        let current_game_id = game_id.lock().unwrap().clone();
        
        // Determine game state
        let has_game_id = !current_game_id.is_empty();
        
        let json_response = if !has_game_id || game.is_none() {
            // Game ID exists but game is not loaded yet (loading)
            json!({
                "status": "",
                "activePlayer": "",
                "isLoaded": false,
                "gameKey": "",
                "boardHtml": ""
            }).to_string()
        } else if let Some(game) = &*game {
            // Game is loaded and ready
            let game_state = game.current_position;
            let active_color = game.active_player;
            let status = match game.current_position.status() {
                BoardStatus::Checkmate => "Checkmate!",
                BoardStatus::Stalemate => "Stalemate",
                BoardStatus::Ongoing => "In progress",
            };
            
            let active_player = match active_color {
                chess::Color::White => "White",
                chess::Color::Black => "Black",
            };
            
            // Generate board HTML
            let mut table = String::new();

            for rank in (0..8).rev() {
                table += &format!("<tr><td class='coord'>{}</td>", rank + 1);
                for file in 0..8 {
                    let square =
                        Square::make_square(Rank::from_index(rank), File::from_index(file));
                    let piece = game_state.piece_on(square);
                    let color = game_state.color_on(square);
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
                            format!("<span class='white-piece'>{}</span>", piece)
                        }
                        Some(chess::Color::Black) => {
                            format!("<span class='black-piece'>{}</span>", piece)
                        }
                        None => piece.to_string(),
                    };

                    let is_dark = (rank + file) % 2 == 0;
                    let cell_class = if is_dark { "dark-square" } else { "light-square" };
                    table += &format!("<td class='{}'>{}</td>", cell_class, piece);
                }
                table += "</tr>";
            }
            table += "<tr><td></td><td class='coord'>a</td><td class='coord'>b</td><td class='coord'>c</td><td class='coord'>d</td><td class='coord'>e</td><td class='coord'>f</td><td class='coord'>g</td><td class='coord'>h</td></tr>";
            
            let board_html = format!("<table>{}</table>", table);
            
            // Use serde_json to create the JSON response
            json!({
                "status": status,
                "activePlayer": active_player,
                "isLoaded": true,
                "gameKey": current_game_id,
                "boardHtml": board_html
            }).to_string()
        } else {
            panic!("Game is not loaded-should not happen");
        };
        
        // Set the content type header to application/json
        let mut response = request.into_response(200, None, &[
            ("Content-Type", "application/json"),
        ])?;
        response.write_all(json_response.as_bytes())?;
        Ok(())
    })?;
    
    Ok(())
}

impl Web {
    pub fn new() -> Web {
        // Create a channel for game ID changes
        Web {
            game: Arc::new(Mutex::new(None)),
            game_key: Arc::new(Mutex::new("".to_string())),
        }
    }

    pub fn register(&self, server: &mut EspHttpServer, event_manager: &EventManager<Event>) -> Result<()> {
        let tx = event_manager.create_sender();
        let rx = event_manager.create_receiver();

        let current_game_for_thread = self.game.clone();
        let game_id_for_thread = self.game_key.clone();
        thread::spawn(move || {
            println!("Starting web event processing thread");
            loop {
                match rx.recv() {
                    Ok(Event::GameState(game_state_event)) => {
                        match game_state_event {
                            GameStateEvent::UpdateGame(expected_physical, game_state) => {
                                current_game_for_thread.lock().unwrap().replace(game_state);
                              //  *game_id_for_thread.lock().unwrap() = expected_physical.to_string();
                            }
                            GameStateEvent::GameLoaded(id) => {
                                // Update the game_id for the /game-info endpoint
                                *game_id_for_thread.lock().unwrap() = id;
                            }
                        }
                    },
                    Ok(_) => continue,
                    Err(e) => {
                        println!("Error receiving game state event: {:?}, exiting thread", e);
                        break;
                    }
                }
            }
            println!("Web event processing thread exited");
        });

        unsafe { 
            handle_js(server)?;
            handle_game(server, self.game_key.clone())?;
            handle_game_data(server, self.game.clone(), self.game_key.clone())?;
            handle_load_game(server, tx, self.game_key.clone())?;
        };

        Ok(())
    }
}
