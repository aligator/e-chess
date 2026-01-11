//! OTA (Over-The-Air) Update Handler
//!
//! Protocol: First message is 4 bytes size as u32 little-endian, following messages are raw binary firmware data.

use crate::bluetooth::{types::*, util::*};
use esp32_nimble::{utilities::mutex::Mutex as NimbleMutex, uuid128, BLEService, NimbleProperties};
use esp_ota::OtaUpdate;
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const OTA_ACTION_CHARACTERISTIC_UUID: &str = "5952abbd-0d7d-4f2d-b0bc-8b3ac5fb8686";
pub const OTA_EVENT_CHARACTERISTIC_UUID: &str = "4d46d598-6141-448c-92bd-fed799efaceb";

#[derive(Debug, Clone)]
pub enum OtaCommand {
    Start { size: u32 },
    Data { data: Vec<u8> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OtaEvent {
    OtaStarted { size: u32 },
    OtaComplete,
    OtaError { message: String },
}

enum OtaState {
    Idle,
    Receiving {
        ota: OtaUpdate,
        expected_size: u32,
        bytes_received: u32,
    },
}

pub struct OtaHandler {}

impl OtaHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn register_characteristics(
        &self,
        service: &Arc<NimbleMutex<BLEService>>,
        ota_command_tx: Sender<OtaCommand>,
    ) -> Result<Arc<NimbleMutex<esp32_nimble::BLECharacteristic>>> {
        let ota_action = service.lock().create_characteristic(
            uuid128!(OTA_ACTION_CHARACTERISTIC_UUID),
            NimbleProperties::WRITE | NimbleProperties::WRITE_ENC,
        );

        let ota_event = service.lock().create_characteristic(
            uuid128!(OTA_EVENT_CHARACTERISTIC_UUID),
            NimbleProperties::READ | NimbleProperties::READ_ENC | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
        );

        {
            let first_message = Arc::new(Mutex::new(true));
            ota_action.lock().on_write(move |args| {
                let data = args.recv_data();

                let mut is_first = first_message.lock().unwrap();

                // First message should be exactly 4 bytes with size
                if *is_first && data.len() == 4 {
                    *is_first = false;
                    let size_bytes: [u8; 4] = [data[0], data[1], data[2], data[3]];
                    let size = u32::from_le_bytes(size_bytes);
                    let _ = ota_command_tx.send(OtaCommand::Start { size });
                    return;
                }

                *is_first = false;

                // All other messages are data chunks
                let _ = ota_command_tx.send(OtaCommand::Data {
                    data: data.to_vec(),
                });
            });
        }

        info!("OTA handler registered");
        Ok(ota_event)
    }

    pub fn start_processor(
        ota_command_rx: std::sync::mpsc::Receiver<OtaCommand>,
        ota_event: Arc<NimbleMutex<esp32_nimble::BLECharacteristic>>,
    ) {
        thread::Builder::new()
            .stack_size(8 * 1024) // 8KB stack for OTA operations
            .spawn(move || {
                info!("OTA processor thread started");
                let state = Arc::new(Mutex::new(OtaState::Idle));

                loop {
                    match ota_command_rx.recv() {
                        Ok(cmd) => {
                            if Self::handle_ota_command(&state, &ota_event, cmd).is_err() {
                                // Reset state on error - no logging to avoid stack overflow
                                if let Ok(mut guard) = state.lock() {
                                    *guard = OtaState::Idle;
                                }
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn OTA processor thread");
    }

    fn handle_ota_command(
        state: &Arc<Mutex<OtaState>>,
        event_char: &Arc<NimbleMutex<esp32_nimble::BLECharacteristic>>,
        cmd: OtaCommand,
    ) -> Result<()> {
        let mut state_guard = state.lock().unwrap();

        match cmd {
            OtaCommand::Start { size } => {
                info!("OTA: Starting update, size={} bytes", size);

                // Begin OTA
                let ota = OtaUpdate::begin()
                    .map_err(|_| BluetoothError::Transport(format!("OTA begin failed")))?;

                *state_guard = OtaState::Receiving {
                    ota,
                    expected_size: size,
                    bytes_received: 0,
                };

                Self::send_event(event_char, OtaEvent::OtaStarted { size })?;

                Ok(())
            }
            OtaCommand::Data { data } => {
                match &mut *state_guard {
                    OtaState::Receiving {
                        ota,
                        expected_size,
                        bytes_received,
                    } => {
                        // Write this chunk
                        ota.write(&data)
                            .map_err(|_| BluetoothError::Transport(format!("Write failed")))?;

                        *bytes_received += data.len() as u32;

                        // Check if all data received
                        if *bytes_received >= *expected_size {
                            info!("OTA: All data received, finalizing");

                            let temp_state = std::mem::replace(&mut *state_guard, OtaState::Idle);

                            if let OtaState::Receiving { ota, .. } = temp_state {
                                let mut completed = ota.finalize().map_err(|_| {
                                    BluetoothError::Transport(format!("Finalize failed"))
                                })?;

                                completed.set_as_boot_partition().map_err(|_| {
                                    BluetoothError::Transport(format!("Set boot failed"))
                                })?;

                                info!("OTA: Success, rebooting soon");

                                Self::send_event(event_char, OtaEvent::OtaComplete)?;

                                thread::spawn(|| {
                                    info!("OTA: Restarting in 2s");
                                    thread::sleep(Duration::from_secs(2));
                                    unsafe {
                                        esp_idf_sys::esp_restart();
                                    }
                                });
                            }
                        }

                        Ok(())
                    }
                    OtaState::Idle => Err(BluetoothError::Protocol("OTA not in progress".into())),
                }
            }
        }
    }

    fn send_event(
        characteristic: &Arc<NimbleMutex<esp32_nimble::BLECharacteristic>>,
        event: OtaEvent,
    ) -> Result<()> {
        let frame = encode_json_frame(&event)?;
        send_chunked_notification(characteristic, &frame);
        Ok(())
    }
}
