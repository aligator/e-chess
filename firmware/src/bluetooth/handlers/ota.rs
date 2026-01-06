//! OTA (Over-The-Air) Update Handler
//!
//! Handles firmware updates over BLE. Uses a binary protocol for data transfer
//! and JSON for control messages.

use crate::bluetooth::{types::*, util::*};
use esp32_nimble::{
    utilities::mutex::Mutex as NimbleMutex, uuid128, BLECharacteristic, BLEService,
    NimbleProperties,
};
use esp_ota::OtaUpdate;
use log::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const OTA_CONTROL_CHARACTERISTIC_UUID: &str = "5952abbd-0d7d-4f2d-b0bc-8b3ac5fb8686";
pub const OTA_DATA_CHARACTERISTIC_UUID: &str = "4d46d598-6141-448c-92bd-fed799efaceb";

/// OTA control messages (JSON protocol)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OtaControlMessage {
    /// Start OTA update (phone -> board)
    OtaStart {
        size: u32,
        chunk_size: u16,
        #[serde(default)]
        checksum: String,
    },
    /// Ready to receive data (board -> phone)
    OtaReady { chunk_size: u16 },
    /// Progress update (board -> phone)
    OtaProgress { bytes_written: u32, total: u32 },
    /// Finalize update (phone -> board)
    OtaFinalize,
    /// Update complete (board -> phone)
    OtaComplete,
    /// Error occurred (board -> phone)
    OtaError { message: String },
}

/// OTA state machine
enum OtaState {
    Idle,
    InProgress {
        ota: OtaUpdate,
        expected_size: u32,
        bytes_written: u32,
        last_sequence: u32,
        expected_checksum: String,
        hasher: Sha256,
    },
}

/// OTA handler manages firmware updates over BLE
pub struct OtaHandler {
    state: Arc<Mutex<OtaState>>,
}

impl OtaHandler {
    /// Create a new OTA handler
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(OtaState::Idle)),
        }
    }

    /// Register OTA characteristics with the BLE service
    pub fn register_characteristics(&self, service: &Arc<NimbleMutex<BLEService>>) -> Result<()> {
        // Control characteristic: board -> phone (notifications) and phone -> board (writes)
        let control_characteristic = service.lock().create_characteristic(
            uuid128!(OTA_CONTROL_CHARACTERISTIC_UUID),
            NimbleProperties::READ
                | NimbleProperties::NOTIFY
                | NimbleProperties::INDICATE
                | NimbleProperties::WRITE,
        );

        // Data characteristic: phone -> board (writes only)
        let data_characteristic = service.lock().create_characteristic(
            uuid128!(OTA_DATA_CHARACTERISTIC_UUID),
            NimbleProperties::WRITE,
        );

        // Setup control message handler
        {
            let state = self.state.clone();
            let control_char = control_characteristic.clone();
            let buffer = Arc::new(Mutex::new(Vec::new()));

            control_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();
                let mut buffer = buffer.lock().unwrap();
                let frames = decode_chunked(data, &mut *buffer);

                for frame in frames {
                    if let Err(e) = Self::handle_control_message(&state, &control_char, &frame) {
                        error!("OTA control error: {:?}", e);
                        let _ = Self::send_control_response(
                            &control_char,
                            OtaControlMessage::OtaError {
                                message: e.to_string(),
                            },
                        );
                        // Reset to idle on error
                        *state.lock().unwrap() = OtaState::Idle;
                    }
                }
            });
        }

        // Setup data chunk handler
        {
            let state = self.state.clone();
            let control_char = control_characteristic.clone();

            data_characteristic.lock().on_write(move |args| {
                let data = args.recv_data();

                // Binary data: first 4 bytes = sequence number (u32 little-endian)
                if data.len() >= 4 {
                    let sequence = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                    let chunk = &data[4..];

                    if let Err(e) = Self::handle_data_chunk(&state, &control_char, sequence, chunk)
                    {
                        error!("OTA data error: {:?}", e);
                        let _ = Self::send_control_response(
                            &control_char,
                            OtaControlMessage::OtaError {
                                message: e.to_string(),
                            },
                        );
                        // Reset to idle on error
                        *state.lock().unwrap() = OtaState::Idle;
                    }
                } else {
                    warn!("OTA data chunk too small: {} bytes", data.len());
                }
            });
        }

        info!("OTA handler registered");
        Ok(())
    }

    fn handle_control_message(
        state: &Arc<Mutex<OtaState>>,
        control_char: &Arc<NimbleMutex<BLECharacteristic>>,
        frame: &[u8],
    ) -> Result<()> {
        let msg: OtaControlMessage = serde_json::from_slice::<Frame<OtaControlMessage>>(frame)
            .map(|f| f.msg)
            .map_err(|e| BluetoothError::Protocol(e.to_string()))?;

        match msg {
            OtaControlMessage::OtaStart {
                size,
                chunk_size,
                checksum,
            } => {
                info!("OTA: Starting update, size={} bytes", size);

                let mut state_guard = state.lock().unwrap();

                // Check if already in progress
                if !matches!(*state_guard, OtaState::Idle) {
                    return Err(BluetoothError::Protocol("OTA already in progress".into()));
                }

                // Initialize OTA
                let ota = OtaUpdate::begin()
                    .map_err(|e| BluetoothError::Transport(format!("OTA begin failed: {:?}", e)))?;

                *state_guard = OtaState::InProgress {
                    ota,
                    expected_size: size,
                    bytes_written: 0,
                    last_sequence: 0,
                    expected_checksum: checksum,
                    hasher: Sha256::new(),
                };

                // Send ready response with negotiated chunk size
                info!("OTA: Sending ready response (chunk_size={})", chunk_size);
                Self::send_control_response(
                    control_char,
                    OtaControlMessage::OtaReady { chunk_size },
                )?;

                info!("OTA: Ready to receive data, waiting for chunks...");
                Ok(())
            }
            OtaControlMessage::OtaFinalize => {
                info!("OTA: Finalizing update");

                let mut state_guard = state.lock().unwrap();

                // Verify we're in the right state and size matches
                if let OtaState::InProgress {
                    bytes_written,
                    expected_size,
                    ..
                } = &*state_guard
                {
                    if *bytes_written != *expected_size {
                        return Err(BluetoothError::Protocol(format!(
                            "Size mismatch: expected {}, got {}",
                            expected_size, bytes_written
                        )));
                    }

                    info!("OTA: Finalizing (wrote {} bytes)", bytes_written);

                    // Take ownership and finalize OTA - this validates and marks the partition
                    // We need to replace the state with Idle to take ownership of ota and hasher
                    let temp_state = std::mem::replace(&mut *state_guard, OtaState::Idle);

                    if let OtaState::InProgress {
                        ota,
                        hasher,
                        expected_checksum,
                        ..
                    } = temp_state
                    {
                        // Validate checksum if provided
                        if !expected_checksum.is_empty() {
                            let calculated_hash = hasher.finalize();
                            let calculated_hex = format!("{:x}", calculated_hash);

                            if calculated_hex != expected_checksum.to_lowercase() {
                                return Err(BluetoothError::Protocol(format!(
                                    "Checksum mismatch: expected {}, got {}",
                                    expected_checksum, calculated_hex
                                )));
                            }

                            info!(
                                "OTA: Checksum validated successfully (SHA256: {})",
                                calculated_hex
                            );
                        } else {
                            warn!("OTA: No checksum provided, skipping validation");
                        }

                        // Finalize OTA partition
                        let mut completed = ota.finalize().map_err(|e| {
                            BluetoothError::Transport(format!("OTA finalize failed: {:?}", e))
                        })?;

                        completed.set_as_boot_partition().map_err(|e| {
                            BluetoothError::Transport(format!("Set boot partition failed: {:?}", e))
                        })?;
                    } else {
                        return Err(BluetoothError::Protocol("Invalid state".into()));
                    }

                    info!("OTA: Update successful, new partition set as boot");

                    // Send complete response
                    Self::send_control_response(control_char, OtaControlMessage::OtaComplete)?;

                    // Reset state
                    *state_guard = OtaState::Idle;

                    // Schedule restart after a delay
                    thread::spawn(|| {
                        info!("OTA: Restarting in 2 seconds...");
                        thread::sleep(Duration::from_secs(2));
                        unsafe {
                            esp_idf_sys::esp_restart();
                        }
                    });

                    Ok(())
                } else {
                    Err(BluetoothError::Protocol("OTA not in progress".into()))
                }
            }
            _ => {
                warn!("OTA: Unexpected control message: {:?}", msg);
                Ok(())
            }
        }
    }

    fn handle_data_chunk(
        state: &Arc<Mutex<OtaState>>,
        control_char: &Arc<NimbleMutex<BLECharacteristic>>,
        sequence: u32,
        chunk: &[u8],
    ) -> Result<()> {
        let mut state_guard = state.lock().unwrap();

        if let OtaState::InProgress {
            ota,
            bytes_written,
            last_sequence,
            expected_size,
            hasher,
            ..
        } = &mut *state_guard
        {
            // Verify sequence number (must be consecutive)
            let expected_seq = *last_sequence + 1;
            if sequence != expected_seq {
                return Err(BluetoothError::Protocol(format!(
                    "Sequence mismatch: expected {}, got {}",
                    expected_seq, sequence
                )));
            }

            // Log first chunk received
            if sequence == 1 {
                info!(
                    "OTA: Received first data chunk (seq={}, {} bytes)",
                    sequence,
                    chunk.len()
                );
            }

            // Update hash with this chunk
            hasher.update(chunk);

            // Write chunk to OTA partition
            ota.write(chunk)
                .map_err(|e| BluetoothError::Transport(format!("OTA write failed: {:?}", e)))?;

            *bytes_written += chunk.len() as u32;
            *last_sequence = sequence;

            // Send progress notification every 64KB or at 10% intervals
            let progress_interval = (*expected_size / 10).max(65536);
            if *bytes_written % progress_interval < chunk.len() as u32 {
                info!(
                    "OTA: Progress {}/{} bytes ({}%)",
                    *bytes_written,
                    *expected_size,
                    (*bytes_written * 100) / *expected_size
                );
                Self::send_control_response(
                    control_char,
                    OtaControlMessage::OtaProgress {
                        bytes_written: *bytes_written,
                        total: *expected_size,
                    },
                )?;
            }

            Ok(())
        } else {
            Err(BluetoothError::Protocol("OTA not in progress".into()))
        }
    }

    fn send_control_response(
        control_char: &Arc<NimbleMutex<BLECharacteristic>>,
        msg: OtaControlMessage,
    ) -> Result<()> {
        let frame = encode_json_frame(&msg)?;
        send_chunked_notification(control_char, &frame);
        Ok(())
    }
}
