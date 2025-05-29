use crate::{
    chess_connector::{ChessConnector, ChessConnectorError},
    event::{GameEvent, OnlineState},
    requester::Requester,
};
use chess::{ChessMove, Game};
use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    sync::mpsc::{self, Sender},
    thread,
};

#[derive(Debug, Serialize, Deserialize)]
struct LichessGameState {
    #[serde(rename = "type")]
    event_type: String,
    moves: String,
    wtime: u64,
    btime: u64,
    winc: u64,
    binc: u64,
    status: String,
    wtakeback: Option<bool>,
    btakeback: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LichessGameResponse {
    id: String,
    #[serde(rename = "initialFen")]
    initial_fen: String,
    state: LichessGameState,
}

enum LichessResponse {
    GameState(LichessGameState),
    Game(LichessGameResponse),
    Other,
}

pub struct LichessConnector<R: Requester> {
    id: Option<String>,

    request: R,
}

impl<R: Requester> LichessConnector<R> {
    pub fn new(request: R) -> Self {
        Self { id: None, request }
    }

    fn create_game(&self, game_response: LichessGameResponse) -> Result<Game, ChessConnectorError> {
        let moves = game_response
            .state
            .moves
            .split(" ")
            .filter(|v| !v.is_empty()) // filter empty strings
            .collect::<Vec<&str>>();

        let mut game = if game_response.initial_fen == "startpos" {
            Game::new()
        } else {
            Game::from_str(&game_response.initial_fen).unwrap()
        };

        for m in moves {
            game.make_move(ChessMove::from_str(m).unwrap());
        }
        Ok(game)
    }

    fn parse_game(&self, game_response: String) -> Result<LichessResponse, ChessConnectorError> {
        // First, try to parse the JSON to get the type field
        let json_value: serde_json::Value = serde_json::from_str(&game_response)
            .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

        // Check if this is a game state update
        if let Some(event_type) = json_value.get("type").and_then(|v| v.as_str()) {
            if event_type == "gameState" {
                // Parse as a game state update
                let game_state: LichessGameState = serde_json::from_value(json_value)
                    .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

                return Ok(LichessResponse::GameState(game_state));
            }
        }

        // Otherwise, try to parse as a regular game response - return Other if it is some other json.
        Ok(serde_json::from_value(json_value)
            .map_or(LichessResponse::Other, |v| LichessResponse::Game(v)))
    }
}

impl<R: Requester> ChessConnector for LichessConnector<R> {
    fn load_game(
        &mut self,
        id: &str,
        game_tx: Sender<GameEvent>,
    ) -> Result<Game, ChessConnectorError> {
        let (tx, rx) = mpsc::channel();

        let url = format!("https://lichess.org/api/board/game/stream/{}", id);
        self.request
            .stream(&mut tx.clone(), &url)
            .map_err(|e| ChessConnectorError::RequestError(e.to_string()))?;

        // Get first response from stream to check if game exists
        let first_response = rx.recv().map_err(|_| ChessConnectorError::GameNotFound)?;

        let response = self.parse_game(first_response)?;
        let game = match response {
            LichessResponse::Game(game) => game,
            _ => {
                return Err(ChessConnectorError::InvalidResponse(
                    "first message is not a valid game response".to_string(),
                ))
            }
        };

        thread::spawn(move || {
            loop {
                let event = rx.recv();
                if let Ok(event) = rx.recv() {
                    // parse_game now handles both game responses and game state updates
                    let response = self.parse_game(event).unwrap();

                    let state = match response {
                        LichessResponse::Game(game) => Some(game.state), // Not sure if this can even happen after the first response...
                        LichessResponse::GameState(state) => Some(state),
                        LichessResponse::Other => None,
                    };

                    if let Some(state) = state {
                        // Get the last move of the event
                        let moves = state
                            .moves
                            .split(" ")
                            .filter(|v| !v.is_empty())
                            .map(|m| m.to_string());

                        game_tx
                            .send(GameEvent::NewOnlineState(OnlineState {
                                moves: moves.collect(),
                                white_request_take_back: state.wtakeback.unwrap_or(false),
                                black_request_take_back: state.btakeback.unwrap_or(false),
                            }))
                            .unwrap();
                    }
                } else {
                    break;
                }
            }
        });

        self.id = Some(id.to_string());

        // Parse json to object
        Ok(self.create_game(game)?)
    }

    fn make_move(&self, chess_move: ChessMove) -> bool {
        if let Some(id) = &self.id {
            // Format move in UCI notation (e.g. "e2e4")
            let move_str = chess_move.to_string();

            // Make move via Lichess API
            let url = format!(
                "https://lichess.org/api/board/game/{}/move/{}",
                id, move_str
            );
            match self.request.post(&url, &move_str) {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}
