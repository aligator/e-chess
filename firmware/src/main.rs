#![deny(warnings)]

use anyhow::Result;
use esp32_nimble::{uuid128, BLEAdvertisementData, BLEDevice, NimbleProperties};
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::log::EspLogger;
use log::info;

fn main() -> Result<()> {
    // Initialize ESP-IDF
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("Starting Bluetooth test");

    let ble_device = BLEDevice::take();
    let ble_advertiser = ble_device.get_advertising();
    let server = ble_device.get_server();

    server.on_connect(|server, clntdesc| {
        // Print connected client data
        println!("{:?}", clntdesc);
        server
            .update_conn_params(clntdesc.conn_handle(), 24, 48, 0, 60)
            .unwrap();
    });
    server.on_disconnect(|_desc, _reason| {
        println!("Disconnected, back to advertising");
        let _ = ble_advertiser.lock().start();
    });

    let my_service = server.create_service(uuid128!("b4d75b6c-7284-4268-8621-6e3cef3c6ac4"));

    let my_service_characteristic = my_service.lock().create_characteristic(
        uuid128!("80580a69-122f-41a8-88c2-8a355fdba6a8"),
        NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::WRITE,
    );

    my_service_characteristic.lock().on_write(|args| {
        println!(
            "current: {:?}, recv: {:?}",
            args.current_data(),
            args.recv_data()
        )
    });

    my_service_characteristic
        .lock()
        .on_read(|characteristic, desc| {
            println!("current: {:?}, recv: {:?}", characteristic, desc)
        });

    my_service_characteristic.lock().set_value(b"Start Value");

    ble_advertiser
        .lock()
        .set_data(
            BLEAdvertisementData::new()
                .name("E-Chess Server")
                .add_service_uuid(my_service.lock().uuid()),
        )
        .unwrap();

    ble_advertiser.lock().start().unwrap();

    let mut val = 0;

    loop {
        FreeRtos::delay_ms(1000);
        my_service_characteristic.lock().set_value(&[val]).notify();
        val = val.wrapping_add(1);
    }

    //Ok(())
}
