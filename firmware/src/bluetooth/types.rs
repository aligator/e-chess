//! Shared types and constants for BLE communication

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;
pub const SERVICE_UUID: &str = "b4d75b6c-7284-4268-8621-6e3cef3c6ac4";

// Keep notifications within the lowest possible BLE ATT MTU (20 bytes -> 23 byte payload).
pub const MIN_MTU_PAYLOAD: usize = 20;

/// Frame wrapper for versioned protocol messages
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Frame<T> {
    pub v: u8,
    #[serde(flatten)]
    pub msg: T,
}

/// Common error type for bluetooth operations
#[derive(Debug)]
pub enum BluetoothError {
    Transport(String),
    Timeout,
    Protocol(String),
}

impl std::fmt::Display for BluetoothError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BluetoothError::Transport(msg) => write!(f, "transport error: {}", msg),
            BluetoothError::Timeout => write!(f, "timeout waiting for response"),
            BluetoothError::Protocol(msg) => write!(f, "protocol error: {}", msg),
        }
    }
}

impl std::error::Error for BluetoothError {}

pub type Result<T> = core::result::Result<T, BluetoothError>;

/// Encode a message into a JSON frame with protocol version and newline terminator
pub fn encode_json_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>> {
    serde_json::to_string(&Frame {
        v: PROTOCOL_VERSION,
        msg,
    })
    .map(|mut body| {
        body.push('\n');
        body.into_bytes()
    })
    .map_err(|e| BluetoothError::Protocol(e.to_string()))
}
