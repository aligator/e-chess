use anyhow::Result;
use chess::{BoardStatus, File, Rank, Square};
use esp_idf_hal::io::Write;
use esp_idf_svc::http::{server::EspHttpServer, Method};
use maud::{html, PreEscaped};
use std::{sync::{mpsc, Arc, Mutex}, thread};

use crate::wifi::page;

#[derive(Debug)]
pub enum GameStateEvent {
    UpdateGame(Option<chess::Game>),
}

#[derive(Debug)]
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
                    div id="game-container" {
                        div id="game-info" class="game-info" {
                            p id="game-status" class="status" { (status) }
                            p id="active-player" class="active-player" { 
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
                        div id="board-container" {}
                        div class="refresh-control" {
                            input type="checkbox" id="autoRefresh" checked="checked" {}
                            label for="autoRefresh" { "Auto refresh" }
                        }
                    }
                    script { (PreEscaped(r#"
                        // Function to fetch and update the board
                        function updateBoard() {
                            fetch('/board-update')
                                .then(response => response.text())
                                .then(html => {
                                    document.getElementById('board-container').innerHTML = html;
                                })
                                .catch(error => {
                                    console.error('Error updating board:', error);
                                });
                        }

                        // Function to fetch and update the game info
                        function updateGameInfo() {
                            fetch('/game-info')
                                .then(response => response.json())
                                .then(data => {
                                    document.getElementById('game-status').textContent = data.status;
                                    document.getElementById('active-player').textContent = 'Active player: ' + data.activePlayer;
                                })
                                .catch(error => {
                                    console.error('Error updating game info:', error);
                                });
                        }

                        // Function to schedule updates
                        function scheduleUpdates() {
                            if (document.getElementById('autoRefresh').checked) {
                                setTimeout(function() {
                                    updateBoard();
                                    updateGameInfo();
                                    scheduleUpdates();
                                }, 1000);
                            }
                        }
                        
                        // Set up event listeners
                        document.getElementById('autoRefresh').addEventListener('change', function() {
                            if (this.checked) {
                                scheduleUpdates();
                            }
                        });

                        document.getElementById('loadGame').addEventListener('click', function() {
                            const gameId = document.getElementById('gameId').value.trim();
                            if (gameId) {
                                fetch('/load-game?id=' + encodeURIComponent(gameId), {
                                    method: 'GET'
                                }).then(function(response) {
                                    if (response.ok) {
                                        updateBoard();
                                        updateGameInfo();
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

                        // Start the update cycle
                        scheduleUpdates();
                    "#)) }
                )
                .into_string(),
            )
        } else {
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
                    p {"No game state"}
                    div id="game-container" {
                        div class="game-id-control" {
                            label for="gameId" { "Game ID: " }
                            input type="text" id="gameId" value=(current_game_id) {}
                            button id="loadGame" { "Load Game" }
                        }
                        div id="board-container" {}
                    }
                    script { (PreEscaped(r#"
                        // Function to fetch and update the board
                        function updateBoard() {
                            fetch('/board-update')
                                .then(response => response.text())
                                .then(html => {
                                    document.getElementById('board-container').innerHTML = html;
                                })
                                .catch(error => {
                                    console.error('Error updating board:', error);
                                });
                        }

                        document.getElementById('loadGame').addEventListener('click', function() {
                            const gameId = document.getElementById('gameId').value.trim();
                            if (gameId) {
                                fetch('/load-game?id=' + encodeURIComponent(gameId), {
                                    method: 'GET'
                                }).then(function(response) {
                                    if (response.ok) {
                                        // After loading a game, redirect to the game page
                                        window.location.href = '/game';
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
                    "#)) }
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

// New function to handle board updates via AJAX
unsafe fn handle_board_update(server: &mut EspHttpServer, game: Arc<Mutex<Option<chess::Game>>>) -> Result<()> {
    server.fn_handler_nonstatic("/board-update", Method::Get, move |request| -> Result<()> {
        let game = game.lock().unwrap();
        
        let html = if let Some(game) = &*game {
            let game_state = game.current_position();
            
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
            
            // Wrap the HTML content with PreEscaped
            format!("<table>{}</table>", table)
        } else {
            // Wrap the HTML content with PreEscaped
            "<p>No game loaded</p>".to_string() 
        };
        
        let mut response = request.into_ok_response()?;
        response.write_all(html.as_bytes())?;
        Ok(())
    })?;
    
    Ok(())
}

// New function to handle game info updates via AJAX
unsafe fn handle_game_info(server: &mut EspHttpServer, game: Arc<Mutex<Option<chess::Game>>>) -> Result<()> {
    server.fn_handler_nonstatic("/game-info", Method::Get, move |request| -> Result<()> {
        let game = game.lock().unwrap();
        
        let json = if let Some(game) = &*game {
            let game_state = game.current_position();
            let active_color = game.side_to_move();
            let status = match game_state.status() {
                BoardStatus::Checkmate => "Checkmate!",
                BoardStatus::Stalemate => "Stalemate",
                BoardStatus::Ongoing => "In progress",
            };
            
            let active_player = match active_color {
                chess::Color::White => "White",
                chess::Color::Black => "Black",
            };
            
            // Properly escape special characters in the JSON string
            let escaped_status = status.replace("\"", "\\\"").replace("\n", "\\n");
            let escaped_player = active_player.replace("\"", "\\\"").replace("\n", "\\n");
            
            format!(r#"{{"status":"{0}","activePlayer":"{1}"}}"#, escaped_status, escaped_player)
        } else {
            r#"{"status":"No game","activePlayer":"None"}"#.to_string()
        };
        
        // Set the content type header to application/json
        let mut response = request.into_response(200, None, &[
            ("Content-Type", "application/json"),
        ])?;
        response.write_all(json.as_bytes())?;
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
            println!("Starting web event processing thread");
            loop {
                match event_rx.recv() {
                    Ok(game_state_event) => {
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
                    Err(e) => {
                        println!("Error receiving game state event: {:?}, exiting thread", e);
                        break;
                    }
                }
            }
            println!("Web event processing thread exited");
        });

        unsafe { 
            handle_favicon(server)?;
            handle_game(server, self.game.clone(), self.game_id.clone())?;
            handle_board_update(server, self.game.clone())?;
            handle_game_info(server, self.game.clone())?;
            handle_load_game(server, tx_cmd)?;
        };

        Ok(rx_cmd)
    }
}
