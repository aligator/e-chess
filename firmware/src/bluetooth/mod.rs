//! Bluetooth LE communication module
//!
//! Implements a modular BLE architecture with separate handlers for:
//! - HTTP Bridge: Proxies HTTP requests from board to phone
//! - Game State: Direct communication for game commands and events
//! - OTA Updates: Over-the-air firmware updates

pub mod handlers;
pub mod types;
pub mod util;

use esp32_nimble::{uuid128, BLEAdvertisementData, BLEDevice};
use handlers::{BridgeHandler, GameHandler, OtaHandler};
use log::*;
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::time::Duration;
use types::*;

use crate::{game::GameStateEvent, Event};

pub struct BluetoothService {
    _device: &'static BLEDevice,
    is_connected: Arc<Mutex<bool>>,
}

impl BluetoothService {
    pub fn new(
        device_name: &str,
        request_timeout: Duration,
        event_tx: Sender<Event>,
    ) -> Result<(Self, BridgeHandler, Sender<GameStateEvent>)> {
        info!("Initializing Bluetooth service");

        let device = BLEDevice::take();
        let server = device.get_server();
        let advertiser = device.get_advertising();

        let is_connected = Arc::new(Mutex::new(false));

        let service = server.create_service(uuid128!(SERVICE_UUID));

        let (bridge_handler, bridge_request_rx, bridge_response_tx) =
            BridgeHandler::new(request_timeout, is_connected.clone());

        let game_handler = GameHandler::new(event_tx.clone());

        let ota_handler = OtaHandler::new();

        let (game_event_tx, game_event_rx) = std::sync::mpsc::channel();
        let (ota_command_tx, ota_command_rx) = std::sync::mpsc::channel();

        let bridge_request_char = bridge_handler.register_characteristics(
            &service,
            bridge_response_tx.clone(),
            bridge_request_rx,
        )?;

        game_handler.register_characteristics(&service, game_event_rx)?;

        let ota_event = ota_handler.register_characteristics(&service, ota_command_tx)?;

        OtaHandler::start_processor(ota_command_rx, ota_event);

        {
            let connection_flag_connect = is_connected.clone();
            server.on_connect(move |server, desc| {
                info!("BLE client connected: {:?}", desc);
                if let Err(e) = server.update_conn_params(desc.conn_handle(), 24, 48, 0, 60) {
                    warn!("Failed to update connection params: {:?}", e);
                }
                *connection_flag_connect.lock().unwrap() = true;
            });
        }

        {
            let connection_flag_disconnect = is_connected.clone();
            let characteristic = bridge_request_char.clone();
            server.on_disconnect(move |_desc, _reason| {
                info!("BLE disconnected, restarting advertising");
                let _ = advertiser.lock().start();
                let _ = characteristic.lock().set_value(b"");
                *connection_flag_disconnect.lock().unwrap() = false;
            });
        }

        bridge_handler.start_dispatcher();

        advertiser
            .lock()
            .set_data(
                BLEAdvertisementData::new()
                    .name(device_name)
                    .add_service_uuid(uuid128!(SERVICE_UUID)),
            )
            .map_err(|e| BluetoothError::Transport(e.to_string()))?;

        advertiser
            .lock()
            .start()
            .map_err(|e| BluetoothError::Transport(e.to_string()))?;

        info!("Bluetooth service initialized and advertising");

        Ok((
            Self {
                _device: device,
                is_connected: is_connected.clone(),
            },
            bridge_handler,
            game_event_tx,
        ))
    }

    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }
}

pub use types::BluetoothError;
