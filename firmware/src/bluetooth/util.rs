//! Utility functions for BLE communication

use super::types::MIN_MTU_PAYLOAD;
use esp32_nimble::{utilities::mutex::Mutex, BLECharacteristic};
use std::sync::Arc;

/// Append incoming bytes to `buffer` and extract complete frames terminated by
/// `\n` or `\r`.
///
/// Behavior:
/// - Incoming `data` is appended to the mutable `buffer`.
/// - The function searches `buffer` for delimiters (`\n` or `\r`). For each
///   delimiter found it drains the slice up to and including the delimiter and
///   returns that drained slice as a `Vec<u8>` (so each returned frame contains
///   the delimiter at the end).
/// - Any trailing bytes in `buffer` after the last delimiter are left in place
///   (these represent a partial frame to be completed by subsequent calls).
///
/// Important notes:
/// - This utility operates on raw bytes (Vec<u8>) and does not attempt UTF-8
///   validation or conversion. Callers must decide how to interpret the bytes
///   (e.g., convert to UTF-8 with lossy replacement if needed).
/// - Frames are returned exactly as drained; the function does not trim
///   whitespace or merge multiple delimiters.
/// - This intentionally mirrors the simple delimiter-based logic used in the
///   BLE `on_write` handler: append, find delimiter position, drain(..=pos),
///   and collect.
pub fn decode_chunked(data: &[u8], buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    // Append new data into the buffer
    buffer.extend_from_slice(data);

    let mut frames: Vec<Vec<u8>> = Vec::new();

    // Drain complete frames/lines using the simple delimiter-based logic.
    while let Some(pos) = buffer.iter().position(|b| *b == b'\n' || *b == b'\r') {
        let frame: Vec<u8> = buffer.drain(..=pos).collect();
        frames.push(frame);
    }

    frames
}

/// Send data in chunks via BLE notification
/// Splits data into MIN_MTU_PAYLOAD sized chunks and sends each as a notification
pub fn send_chunked_notification(characteristic: &Arc<Mutex<BLECharacteristic>>, data: &[u8]) {
    use log::*;
    info!(
        "send_chunked_notification: sending {} bytes in {} chunks",
        data.len(),
        data.chunks(MIN_MTU_PAYLOAD).count()
    );
    for (i, chunk) in data.chunks(MIN_MTU_PAYLOAD).enumerate() {
        let mut chr_lock = characteristic.lock();
        chr_lock.set_value(chunk);
        chr_lock.notify();
        info!("  chunk {}: sent {} bytes via notify()", i, chunk.len());
    }
}
