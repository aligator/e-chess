//! OTA (Over-The-Air) Update Handler
//!
//! Protocol: First bytes until space are ASCII size, rest is binary firmware data.

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
    Start { size: u32, data: Vec<u8> },
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
    InProgress {
        ota: OtaUpdate,
        expected_size: u32,
        bytes_written: u32,
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
            NimbleProperties::WRITE,
        );

        let ota_event = service.lock().create_characteristic(
            uuid128!(OTA_EVENT_CHARACTERISTIC_UUID),
            NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::INDICATE,
        );

        {
            ota_action.lock().on_write(move |args| {
                let data = args.recv_data();

                if let Some(space_pos) = data.iter().position(|&b| b == b' ') {
                    if let Ok(size_str) = std::str::from_utf8(&data[..space_pos]) {
                        if let Ok(size) = size_str.parse::<u32>() {
                            let firmware_data = data[space_pos + 1..].to_vec();
                            let _ = ota_command_tx.send(OtaCommand::Start {
                                size,
                                data: firmware_data,
                            });
                            return;
                        }
                    }
                }

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
        thread::spawn(move || {
            info!("OTA processor thread started");
            let state = Arc::new(Mutex::new(OtaState::Idle));

            while let Ok(cmd) = ota_command_rx.recv() {
                if let Err(e) = Self::handle_ota_command(&state, &ota_event, cmd) {
                    error!("OTA error: {:?}", e);
                    let _ = Self::send_event(
                        &ota_event,
                        OtaEvent::OtaError {
                            message: e.to_string(),
                        },
                    );
                    *state.lock().unwrap() = OtaState::Idle;
                }
            }
            info!("OTA processor thread exiting");
        });
    }

    fn handle_ota_command(
        state: &Arc<Mutex<OtaState>>,
        event_char: &Arc<NimbleMutex<esp32_nimble::BLECharacteristic>>,
        cmd: OtaCommand,
    ) -> Result<()> {
        let mut state_guard = state.lock().unwrap();

        match cmd {
            OtaCommand::Start { size, data } => {
                info!("OTA: Starting update, size={} bytes", size);

                let ota = OtaUpdate::begin()
                    .map_err(|e| BluetoothError::Transport(format!("OTA begin failed: {:?}", e)))?;

                let bytes_written = data.len() as u32;

                if !data.is_empty() {
                    info!("OTA: Writing initial chunk of {} bytes", bytes_written);
                }

                let mut ota_update = ota;
                if !data.is_empty() {
                    ota_update.write(&data).map_err(|e| {
                        BluetoothError::Transport(format!("OTA write failed: {:?}", e))
                    })?;
                }

                *state_guard = OtaState::InProgress {
                    ota: ota_update,
                    expected_size: size,
                    bytes_written,
                };

                Self::send_event(event_char, OtaEvent::OtaStarted { size })?;

                Ok(())
            }
            OtaCommand::Data { data } => {
                if let OtaState::InProgress {
                    ota,
                    bytes_written,
                    expected_size,
                } = &mut *state_guard
                {
                    ota.write(&data).map_err(|e| {
                        BluetoothError::Transport(format!("OTA write failed: {:?}", e))
                    })?;

                    *bytes_written += data.len() as u32;

                    if *bytes_written >= *expected_size {
                        info!("OTA: All data received, finalizing update");

                        let temp_state = std::mem::replace(&mut *state_guard, OtaState::Idle);

                        if let OtaState::InProgress { ota, .. } = temp_state {
                            let mut completed = ota.finalize().map_err(|e| {
                                BluetoothError::Transport(format!("OTA finalize failed: {:?}", e))
                            })?;

                            completed.set_as_boot_partition().map_err(|e| {
                                BluetoothError::Transport(format!(
                                    "Set boot partition failed: {:?}",
                                    e
                                ))
                            })?;

                            info!("OTA: Update successful, new partition set as boot");

                            Self::send_event(event_char, OtaEvent::OtaComplete)?;

                            thread::spawn(|| {
                                info!("OTA: Restarting in 2 seconds...");
                                thread::sleep(Duration::from_secs(2));
                                unsafe {
                                    esp_idf_sys::esp_restart();
                                }
                            });
                        }
                    }

                    Ok(())
                } else {
                    Err(BluetoothError::Protocol("OTA not in progress".into()))
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
