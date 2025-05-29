use chess::BitBoard;

#[derive(PartialEq)]
pub struct OnlineState {
    pub white_request_take_back: bool,
    pub black_request_take_back: bool,
    pub moves: Vec<String>,
}

#[derive(PartialEq)]
pub enum GameEvent {
    NewOnlineState(OnlineState),
    NewPhysicalState(BitBoard),
    None,
}
