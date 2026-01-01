use crate::{
    chess_connector::{
        ChessConnector, ChessConnectorError, GameEvent, GameState, OngoingGame, PlayerInfo,
    },
    requester::Requester,
};
use chess::{ChessMove, Game};
use serde::{Deserialize, Serialize};
use std::{
    str::FromStr,
    sync::mpsc::{self, Receiver, Sender},
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

    upstream_rx: Receiver<String>,
    upstream_tx: Sender<String>,
}

impl<R: Requester> LichessConnector<R> {
    pub fn new(request: R) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            id: None,
            request,
            upstream_rx: rx,
            upstream_tx: tx,
        }
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

fn map_ongoing_game(json_value: &serde_json::Value) -> Option<OngoingGame> {
    let game_id = json_value.get("gameId");
    let oppenent_info = json_value.get("opponent");

    if game_id.is_none() || oppenent_info.is_none() {
        return None;
    }

    let opponent = oppenent_info.unwrap();
    let id = opponent.get("id")?.as_str()?.to_string();
    let username = opponent.get("username")?.as_str()?.to_string();

    if let Some(game_id) = game_id.and_then(|v| v.as_str()) {
        Some(OngoingGame {
            game_id: game_id.to_string(),
            opponent: PlayerInfo { id, username },
        })
    } else {
        None
    }
}

impl<R: Requester> ChessConnector for LichessConnector<R> {
    fn find_open_games(&self) -> Result<Vec<OngoingGame>, ChessConnectorError> {
        // Call lichess API to get list of open games
        let response = self
            .request
            .get("https://lichess.org/api/account/playing?nb=9")
            .map_err(|e| ChessConnectorError::RequestError(e.to_string()))?;

        // First, try to parse the JSON to get the type field
        let json_value: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

        // Map the ids of the open games
        let now_playing = json_value
            .get("nowPlaying")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ChessConnectorError::InvalidResponse(
                    "missing or invalid 'nowPlaying' field".to_string(),
                )
            })?;

        let game_ids = now_playing
            .iter()
            .filter_map(|game| map_ongoing_game(game))
            .collect::<Vec<OngoingGame>>();
        Ok(game_ids)
    }

    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError> {
        let (tx, rx) = mpsc::channel();
        self.upstream_rx = rx;
        self.upstream_tx = tx;

        let url = format!("https://lichess.org/api/board/game/stream/{}", id);
        self.request
            .stream(&mut self.upstream_tx.clone(), &url)
            .map_err(|e| ChessConnectorError::RequestError(e.to_string()))?;

        // Get first response from stream to check if game exists
        let first_response = self
            .upstream_rx
            .recv()
            .map_err(|_| ChessConnectorError::GameNotFound)?;

        let response = self.parse_game(first_response)?;
        let game = match response {
            LichessResponse::Game(game) => game,
            _ => {
                return Err(ChessConnectorError::InvalidResponse(
                    "first message is not a valid game response".to_string(),
                ))
            }
        };

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

    fn next_event(&self) -> Result<Option<GameEvent>, ChessConnectorError> {
        match self.upstream_rx.try_recv() {
            Ok(event) => {
                // parse_game now handles both game responses and game state updates
                let response = self.parse_game(event)?;

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

                    Ok(Some(GameEvent::State(GameState {
                        moves: moves.collect(),
                        white_request_take_back: state.wtakeback.unwrap_or(false),
                        black_request_take_back: state.btakeback.unwrap_or(false),
                    })))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    fn is_connected(&self) -> bool {
        self.request.is_connected()
    }

    fn is_valid_key(&self, key: String) -> bool {
        let len = key.len();
        let valid = len >= 8 && len <= 12 && key.chars().all(|c| c.is_ascii_alphanumeric());
        valid
    }
}
