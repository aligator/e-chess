use anyhow::bail;
use anyhow::Result;
use board::Board;
use chess_game::chess_connector::LocalChessConnector;
use chess_game::game::ChessGame;
use core::str;
use embedded_svc::{
    http::{client::Client, Method},
    io::Read,
};
use esp_idf_hal::i2c::*;
use esp_idf_hal::io::Write;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use log::*;
use maud::html;
use std::thread::sleep;
use std::time::Duration;
use storage::Storage;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

mod board;
mod constants;
mod display;
mod storage;
mod web;
mod wifi;

fn get(url: impl AsRef<str>, token: Option<impl AsRef<str>>) -> Result<()> {
    let mut config = Configuration::default();
    config.use_global_ca_store = true;
    config.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    let connection = EspHttpConnection::new(&config)?;
    let mut client = Client::wrap(connection);

    // 3. Open a GET request to `url`

    let headers = if let Some(token) = token {
        [
            ("accept", "text/plain"),
            ("Authorization", &format!("Bearer {}", token.as_ref())),
        ]
    } else {
        [("accept", "text/plain"), ("", "")]
    };

    // ANCHOR: request
    let request = client.request(Method::Get, url.as_ref(), &headers)?;
    // ANCHOR_END: request

    // 4. Submit the request and check the status code of the response.
    // Successful http status codes are in the 200..=299 range.
    let response = request.submit()?;
    let status = response.status();
    println!("Response code: {}\n", status);
    match status {
        200..=299 => {
            // 5. If the status is OK, read response data chunk by chunk into a buffer and print it until done.
            //
            // NB. There is no guarantee that chunks will be split at the boundaries of valid UTF-8
            // sequences (in fact it is likely that they are not) so this edge case needs to be handled.
            // However, for the purposes of clarity and brevity(?), the additional case of completely invalid
            // UTF-8 sequences will not be handled here and is left as an exercise for the reader.
            let mut buf = [0_u8; 256];
            // Offset into the buffer to indicate that there may still be
            // bytes at the beginning that have not been decoded yet
            let mut offset = 0;
            // Keep track of the total number of bytes read to print later
            let mut total = 0;
            let mut reader = response;
            loop {
                // read into the buffer starting at the offset to not overwrite
                // the incomplete UTF-8 sequence we put there earlier
                if let Ok(size) = Read::read(&mut reader, &mut buf[offset..]) {
                    if size == 0 {
                        // It might be nice to check if we have any left over bytes here (ie. the offset > 0)
                        // as this would mean that the response ended with an invalid UTF-8 sequence, but for the
                        // purposes of this training we are assuming that the full response will be valid UTF-8
                        break;
                    }
                    // Update the total number of bytes read
                    total += size;
                    // 6. Try converting the bytes into a Rust (UTF-8) string and print it.
                    // Remember that we read into an offset and recalculate the real length
                    // of the bytes to decode.
                    let size_plus_offset = size + offset;
                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            // buffer contains fully valid UTF-8 data,
                            // print it and reset the offset to 0.
                            print!("{}", text);
                            offset = 0;
                        }
                        Err(error) => {
                            // The buffer contains incomplete UTF-8 data, we will
                            // print the valid part, copy the invalid sequence to
                            // the beginning of the buffer and set an offset for the
                            // next read.
                            //
                            // NB. There is actually an additional case here that should be
                            // handled in a real implementation. The Utf8Error may also contain
                            // an error_len field indicating that there is actually an invalid UTF-8
                            // sequence in the middle of the buffer. Such an error would not be
                            // recoverable through our offset and copy mechanism. The result will be
                            // that the invalid sequence will be copied to the front of the buffer and
                            // eventually the buffer will be filled until no more bytes can be read when
                            // the offset == buf.len(). At this point the loop will exit without reading
                            // any more of the response.
                            let valid_up_to = error.valid_up_to();
                            unsafe {
                                // It's ok to use unsafe here as the error code already told us that
                                // the UTF-8 data up to this point is valid, so we can tell the compiler
                                // it's fine.
                                print!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                            }
                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                        }
                    }
                }
            }
            println!("Total: {} bytes", total);
        }
        _ => bail!("Unexpected response code: {}", status),
    }

    Ok(())
}

/// Entry point to our application.
fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    info!("Starting E-Chess!");

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mcp23017: I2cDriver<'_> = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let mut chess: ChessGame<LocalChessConnector> = ChessGame::new(LocalChessConnector::new(), "")?;

    let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio23)?;

    #[cfg(not(feature = "no_board"))]
    let mut board = Board::new(mcp23017, 0x20);
    #[cfg(not(feature = "no_board"))]
    board.setup()?;

    let mut display = display::Display::new(ws2812);
    display.setup()?;

    let mut web = web::Web::new();

    let nvs = EspDefaultNvsPartition::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs.clone()))?;
    let storage = Storage::new(nvs.clone())?;

    let token = storage.get_str::<25>("api_token")?;
    info!("API Token: {:?}", token);

    let mut server = wifi::start_wifi(wifi_driver, storage)?;

    server.fn_handler("/", Method::Get, |request| {
        let html = wifi::page(
            html!(
                h1 { "E-Chess" }
                p { "Welcome to E-Chess!" }
                a href="/settings" { "Settings" }
                a href="/game" { "Game" }
            )
            .into_string(),
        );

        request.into_ok_response()?.write_all(html.as_bytes())
    })?;

    web.register(&mut server)?;

    match get(
        "https://lichess.org/api/board/game/stream/CO13DXqoWvSo",
        token,
    ) {
        Ok(response) => {
            info!("Response: {:?}", response);
        }
        Err(err) => {
            error!("Error: {:?}", err);
        }
    }

    // Start the main loop
    info!("Start app loop");
    loop {
        #[cfg(not(feature = "no_board"))]
        match board.tick() {
            Ok(physical) => {
                let _ = chess.tick(physical);
                web.tick(chess.game.clone());
                display.tick(physical, &chess)?;
            }
            Err(e) => {
                error!("Error: {:?}", e);
            }
        }

        #[cfg(feature = "no_board")]
        {
            let game = chess::Game::default();
            let _ = chess.tick(*game.current_position().combined());
            web.tick(chess.game.clone());
            display.tick(*game.current_position().combined(), &chess)?;
        }

        sleep(Duration::from_millis(1000));
    }
}
