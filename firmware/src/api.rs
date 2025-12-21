use std::sync::{Arc, Mutex};

use chess_game::{
    chess_connector::{ChessConnector, LocalChessConnector},
    lichess::LichessConnector,
};

use crate::{game::Settings, request::EspRequester};

pub fn create(settings: Arc<Mutex<Settings>>) -> Box<dyn ChessConnector> {
    let api_token = {
        // Scope to release the lock early.
        &settings.lock().unwrap().token.clone()
    };

    if api_token.is_empty() {
        LocalChessConnector::new()
    } else {
        let requester = EspRequester::new(api_token.to_string());
        Box::new(LichessConnector::new(requester))
    }
}
