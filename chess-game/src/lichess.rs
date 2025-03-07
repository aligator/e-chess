use std::sync::mpsc::{self, Receiver, Sender};

use crate::{
    chess_connector::{ChessConnector, ChessConnectorError},
    requester::Requester,
};
use chess::ChessMove;

pub struct LichessConnector<R: Requester> {
    request: R,

    upstream_rx: Receiver<String>,
    upstream_tx: Sender<String>,
}

impl<R: Requester> LichessConnector<R> {
    pub fn new(request: R) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            request,
            upstream_rx: rx,
            upstream_tx: tx,
        }
    }
}

impl<R: Requester> LichessConnector<R> {
    fn response_to_fen(&self, response: String) -> Result<String, ChessConnectorError> {}
}

impl<R: Requester> ChessConnector for LichessConnector<R> {
    fn load_game(&self, id: &str) -> Result<String, ChessConnectorError> {
        let url = format!("https://lichess.org/api/board/game/stream/{}", id);
        self.request.stream(&mut self.upstream_tx.clone(), &url)?;

        // Get first response from stream to check if game exists
        let first_response = self
            .upstream_rx
            .recv()
            .map_err(|_| ChessConnectorError::GameNotFound)?;
        if first_response.contains("error") {
            return Err(ChessConnectorError::GameNotFound);
        }

        println!("{}", first_response);

        Ok(self.response_to_fen(first_response)?)
    }

    fn make_move(&self, chess_move: ChessMove) -> bool {
        true
    }

    fn tick(&self) -> Result<String, ChessConnectorError> {
        match self.upstream_rx.try_recv() {
            Ok(event) => Ok(self.response_to_fen(event)?),
            Err(_) => Ok(String::new()),
        }
    }
}
