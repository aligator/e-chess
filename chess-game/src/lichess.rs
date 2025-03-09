use crate::{
    chess_connector::{ChessConnector, ChessConnectorError},
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
}

#[derive(Debug, Serialize, Deserialize)]
struct LichessGameResponse {
    id: String,
    #[serde(rename = "initialFen")]
    initial_fen: String,
    state: LichessGameState,
}

enum LichessEvent {
    GameResponse(LichessGameResponse),
    GameStateUpdate(LichessGameState),
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

    fn create_game(
        &self,
        game_response: LichessGameResponse,
    ) -> Result<Game, ChessConnectorError<R>> {
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

    fn parse_game(
        &self,
        game_response: String,
    ) -> Result<LichessGameResponse, ChessConnectorError<R>> {
        // First, try to parse the JSON to get the type field
        let json_value: serde_json::Value = serde_json::from_str(&game_response)
            .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

        // Check if this is a game state update
        if let Some(event_type) = json_value.get("type").and_then(|v| v.as_str()) {
            if event_type == "gameState" {
                // Parse as a game state update
                let game_state: LichessGameState = serde_json::from_value(json_value)
                    .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

                // Create a GameResponse with the necessary fields
                let game = LichessGameResponse {
                    id: self.id.clone().unwrap_or_default(),
                    initial_fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
                        .to_string(),
                    state: LichessGameState {
                        event_type: "gameState".to_string(),
                        moves: game_state.moves,
                        wtime: 0,
                        btime: 0,
                        winc: 0,
                        binc: 0,
                        status: "".to_string(),
                    },
                };

                return Ok(game);
            }
        }

        // Otherwise, try to parse as a regular game response
        let game: LichessGameResponse = serde_json::from_value(json_value)
            .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

        Ok(game)
    }
}

impl<R: Requester> ChessConnector for LichessConnector<R> {
    type R = R;

    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError<R>> {
        let url = format!("https://lichess.org/api/board/game/stream/{}", id);
        self.request
            .stream(&mut self.upstream_tx.clone(), &url)
            .map_err(|e| ChessConnectorError::<R>::RequestError(e))?;

        // Get first response from stream to check if game exists
        let first_response = self
            .upstream_rx
            .recv()
            .map_err(|_| ChessConnectorError::GameNotFound)?;

        let game = self.parse_game(first_response)?;

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

    fn tick(&self) -> Result<Option<String>, ChessConnectorError<R>> {
        match self.upstream_rx.try_recv() {
            Ok(event) => {
                // parse_game now handles both game responses and game state updates
                let game = self.parse_game(event)?;

                // Get the last move
                let last_move = game
                    .state
                    .moves
                    .split(" ")
                    .filter(|v| !v.is_empty())
                    .last()
                    .unwrap_or_default();

                // Only make a move if we have one
                if !last_move.is_empty() {
                    // Make the move
                    match ChessMove::from_str(last_move) {
                        Ok(chess_move) => {
                            self.make_move(chess_move);
                        }
                        Err(e) => {
                            return Err(ChessConnectorError::InvalidResponse(e.to_string()));
                        }
                    }

                    // TODO validate if it was the only move since the last tick?

                    Ok(Some(last_move.to_string()))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }
}
