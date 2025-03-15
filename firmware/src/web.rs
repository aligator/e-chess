use anyhow::Result;
use chess::{BoardStatus, File, Game, Rank, Square};
use esp_idf_hal::io::Write;
use esp_idf_svc::http::{server::EspHttpServer, Method};
use maud::{html, PreEscaped};
use std::{sync::{mpsc, Arc, Mutex}, thread};

use crate::wifi::page;

pub enum GameStateEvent {
    UpdateGame(Option<chess::Game>),
}

pub enum GameCommandEvent {
    LoadNewGame(String),
}

pub struct Web {
    game: Arc<Mutex<Option<chess::Game>>>,
    game_id: Arc<Mutex<String>>,
}

unsafe fn handle_game(server: &mut EspHttpServer, game: Arc<Mutex<Option<chess::Game>>>, current_game_id: Arc<Mutex<String>>) -> Result<()> {


    server.fn_handler_nonstatic("/game", Method::Get, move |request| {
        let game = game.lock().unwrap();
        let current_game_id = current_game_id.lock().unwrap().clone();

        let html = if let Some(game) = &*game {
            let game_state = game.current_position();
            let active_color = game.side_to_move();
            let status = match game_state.status() {
                BoardStatus::Checkmate => "Checkmate!",
                BoardStatus::Stalemate => "Stalemate",
                BoardStatus::Ongoing => "In progress",
            };

            let mut table: String = String::new();

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
            
            page(
                html!(
                    style { r#"
                        body { 
                            font-family: Arial, sans-serif;
                            display: flex;
                            flex-direction: column;
                            align-items: center;
                            background-color: #f0f0f0;
                        }
                        h1 { 
                            color: #333;
                            margin-bottom: 1em;
                        }
                        table {
                            border-collapse: collapse;
                            margin: 20px;
                        }
                        td {
                            width: 50px;
                            height: 50px;
                            text-align: center;
                            font-size: 2em;
                        }
                        .coord {
                            font-size: 1em;
                            padding: 5px;
                            color: #666;
                        }
                        .dark-square {
                            background-color: #b58863;
                        }
                        .light-square {
                            background-color: #f0d9b5;
                        }
                        .white-piece {
                            color: #fff;
                            text-shadow: 0 0 2px #000;
                        }
                        .black-piece {
                            color: #000;
                            text-shadow: 0 0 2px #fff;
                        }
                        .refresh-control {
                            margin: 20px;
                            display: flex;
                            align-items: center;
                            gap: 10px;
                            font-family: Arial, sans-serif;
                        }
                        .refresh-control input {
                            width: 20px;
                            height: 20px;
                        }
                        .game-info {
                            margin: 20px;
                            padding: 15px;
                            background-color: white;
                            border-radius: 8px;
                            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                        }
                        .game-info p {
                            margin: 5px 0;
                            font-size: 1.1em;
                        }
                        .status {
                            font-weight: bold;
                            color: #d63031;
                        }
                        .active-player {
                            color: #2d3436;
                        }
                        .game-id-control {
                            margin: 20px;
                            display: flex;
                            align-items: center;
                            gap: 10px;
                            font-family: Arial, sans-serif;
                        }
                        .game-id-control input[type="text"] {
                            padding: 8px;
                            border: 1px solid #ccc;
                            border-radius: 4px;
                            font-size: 1em;
                        }
                        .game-id-control button {
                            padding: 8px 16px;
                            background-color: #2980b9;
                            color: white;
                            border: none;
                            border-radius: 4px;
                            cursor: pointer;
                            font-size: 1em;
                        }
                        .game-id-control button:hover {
                            background-color: #3498db;
                        }
                    "# }
                    h1 { "E-Chess" }
                    div class="game-info" {
                        p class="status" { (status) }
                        p class="active-player" { 
                            "Active player: " 
                            (match active_color {
                                chess::Color::White => "White",
                                chess::Color::Black => "Black",
                            })
                        }
                    }
                    div class="game-id-control" {
                        label for="gameId" { "Game ID: " }
                        input type="text" id="gameId" value=(current_game_id) {}
                        button id="loadGame" { "Load Game" }
                    }
                    table { (PreEscaped(table)) }
                    div class="refresh-control" {
                        input type="checkbox" id="autoRefresh" checked="checked" {}
                        label for="autoRefresh" { "Auto refresh" }
                    }
                    script { r#"
                        function scheduleRefresh() {
                            if (document.getElementById('autoRefresh').checked) {
                                setTimeout(function() {
                                    location.reload();
                                }, 1000);
                            }
                        }
                        
                        scheduleRefresh();
                        
                        document.getElementById('autoRefresh').addEventListener('change', function() {
                            scheduleRefresh();
                        });

                        document.getElementById('loadGame').addEventListener('click', function() {
                            const gameId = document.getElementById('gameId').value.trim();
                            if (gameId) {
                                fetch('/load-game?id=' + encodeURIComponent(gameId), {
                                    method: 'GET'
                                }).then(function(response) {
                                    if (response.ok) {
                                        location.reload();
                                    } else {
                                        alert('Failed to load game. Please check the game ID.');
                                    }
                                }).catch(function(error) {
                                    alert('Error: ' + error);
                                });
                            } else {
                                alert('Please enter a valid game ID');
                            }
                        });
                    "# }
                )
                .into_string(),
            )
        } else {
            page(
                html!(
                    h1 { "E-Chess" }
                    p {"No game state"}
                    div class="game-id-control" {
                        label for="gameId" { "Game ID: " }
                        input type="text" id="gameId" value=(current_game_id) {}
                        button id="loadGame" { "Load Game" }
                    }
                    script { r#"
                        document.getElementById('loadGame').addEventListener('click', function() {
                            const gameId = document.getElementById('gameId').value.trim();
                            if (gameId) {
                                fetch('/load-game?id=' + encodeURIComponent(gameId), {
                                    method: 'GET'
                                }).then(function(response) {
                                    if (response.ok) {
                                        location.reload();
                                    } else {
                                        alert('Failed to load game. Please check the game ID.');
                                    }
                                }).catch(function(error) {
                                    alert('Error: ' + error);
                                });
                            } else {
                                alert('Please enter a valid game ID');
                            }
                        });
                    "# }
                )
                .into_string(),
            )
        };
        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    Ok(())
}

unsafe fn handle_favicon(server: &mut EspHttpServer) -> Result<()> {
    server.fn_handler_nonstatic("/favicon.ico", Method::Get, move |request| -> Result<()> {
        // Include the favicon file at compile time
        const FAVICON: &[u8] = include_bytes!("../assets/favicon.ico");

        let mut response = request.into_ok_response()?;
        response.write_all(FAVICON)?;
        Ok(())
    })?;
    Ok(())
}

unsafe fn handle_load_game(server: &mut EspHttpServer, sender: mpsc::Sender<GameCommandEvent>) -> Result<()> {
    server.fn_handler_nonstatic("/load-game", Method::Get, move |request| -> Result<()> {
        let uri = request.uri();
        
        // Parse the query string to get the game ID
        if let Some(query) = uri.split('?').nth(1) {
            if let Some(id_param) = query.split('&').find(|p| p.starts_with("id=")) {
                if let Some(id) = id_param.split('=').nth(1) {
                    println!("Loading game: {}", id);
                    // Send the game ID through the channel
                    sender.send(GameCommandEvent::LoadNewGame(id.to_string())).unwrap();
                }
            }
        }
        
        let mut response = request.into_ok_response()?;
        response.write_all(b"OK")?;
        Ok(())
    })?;
    
    Ok(())
}

impl Web {
    pub fn new() -> Web {
        // Create a channel for game ID changes
        Web {
            game: Arc::new(Mutex::new(None)),
            game_id: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn register(&self, server: &mut EspHttpServer, event_rx: mpsc::Receiver<GameStateEvent>) -> Result<mpsc::Receiver<GameCommandEvent>> {
        let (tx_cmd, rx_cmd) = mpsc::channel::<GameCommandEvent>();

        let current_game_for_thread = self.game.clone();
        thread::spawn(move || {
            if let Ok(game_state_event) = event_rx.recv() {
                match game_state_event {
                    GameStateEvent::UpdateGame(updated_game) => {
                        if let Some(updated_game) = updated_game {
                            current_game_for_thread.lock().unwrap().replace(updated_game);
                        } else {
                            current_game_for_thread.lock().unwrap().take();
                        }
                    }
                }
            }
        });


        println!("Registering Web");
        unsafe { 
            handle_favicon(server)?;
            handle_game(server, self.game.clone(), self.game_id.clone())?;
            handle_load_game(server, tx_cmd)?;
        };

        Ok(rx_cmd)
    }
}
