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
    moves: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LichessGameResponse {
    id: String,
    #[serde(rename = "initialFen")]
    initial_fen: String,
    state: LichessGameState,
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

    fn parse_game(
        &self,
        game_response: String,
    ) -> Result<LichessGameResponse, ChessConnectorError> {
        let game: LichessGameResponse = serde_json::from_str(&game_response)
            .map_err(|e| ChessConnectorError::InvalidResponse(e.to_string()))?;

        Ok(game)
    }
}

impl<R: Requester> ChessConnector for LichessConnector<R> {
    fn load_game(&mut self, id: &str) -> Result<Game, ChessConnectorError> {
        let url = format!("https://lichess.org/api/board/game/stream/{}", id);
        self.request.stream(&mut self.upstream_tx.clone(), &url)?;

        // Get first response from stream to check if game exists
        let first_response = self
            .upstream_rx
            .recv()
            .map_err(|_| ChessConnectorError::GameNotFound)?;

        println!("{}", first_response);
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

    fn tick(&self) -> Result<Option<String>, ChessConnectorError> {
        match self.upstream_rx.try_recv() {
            Ok(event) => {
                let game = self.parse_game(event)?;
                // Get the last move
                let last_move = game
                    .state
                    .moves
                    .split(" ")
                    .filter(|v| !v.is_empty())
                    .last()
                    .unwrap();
                // Make the move
                self.make_move(ChessMove::from_str(last_move).unwrap());

                // TODO validate if it was the only move since the last tick?

                Ok(Some(last_move.to_string()))
            }
            Err(_) => Ok(None),
        }
    }
}
