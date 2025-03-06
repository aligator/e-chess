use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use chess::ChessMove;
use chess_game::{
    chess_connector::{ChessConnector, ChessConnectorError},
    request::Request,
};

pub struct LichessConnector<R: Request> {
    request: R,
}

impl<R: Request> LichessConnector<R> {
    pub fn new(request: R) -> Self {
        Self { request }
    }
}

impl<R: Request> ChessConnector for LichessConnector<R> {
    fn load_game(&self, id: &str) -> Result<String, ChessConnectorError> {
        let (mut tx, rx) = mpsc::channel();

        let url = format!("https://lichess.org/api/game/{}", id);
        self.request.stream(&mut tx, &url)?;

        // Run the request in a new thread
        thread::spawn(move || {
            while let Ok(response) = rx.recv() {
                println!("{}", response);
            }
        });

        Ok(String::new())
    }

    fn make_move(&self, chess_move: ChessMove) -> bool {
        true
    }
}
