use chess_game::requester::Requester;
use core::str;
use embedded_svc::{
    http::{client::Client, Method},
    io::Read,
};
use esp_idf_hal::io::{EspIOError, Write};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_sys::EspError;
use log::*;
use std::error::Error;
use std::fmt;
use std::sync::mpsc::{RecvError, Sender};
use std::thread::{self};

#[derive(Debug)]
pub enum RequestError {
    Esp(EspError),
    EspIO(EspIOError),
    Status(u16),
    Read(String),
    Recv(RecvError),
}

impl Error for RequestError {}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Esp(e) => write!(f, "ESP error: {:?}", e),
            RequestError::EspIO(e) => write!(f, "ESP IO error: {:?}", e),
            RequestError::Status(code) => write!(f, "HTTP status error: {}", code),
            RequestError::Read(msg) => write!(f, "Read error: {}", msg),
            RequestError::Recv(e) => write!(f, "Receive error: {:?}", e),
        }
    }
}

/// Helper to create a configured HTTP client
///
/// It is separately to make it more easy to use it inside a thread.
fn create_client() -> Result<Client<EspHttpConnection>, RequestError> {
    let mut config = Configuration::default();
    config.use_global_ca_store = true;
    config.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    match EspHttpConnection::new(&config) {
        Ok(connection) => Ok(Client::wrap(connection)),
        Err(e) => Err(RequestError::Esp(e)),
    }
}

// ESP implementation of the Requester trait
#[derive(Debug)]
pub struct EspRequester {
    api_key: String,
}

impl EspRequester {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    // Helper to process HTTP response to string
    fn process_response(mut response: impl Read, status: u16) -> Result<String, RequestError> {
        if !(200..=299).contains(&status) {
            return Err(RequestError::Status(status));
        }

        let mut buf = [0_u8; 256];
        let mut offset = 0;
        let mut total = 0;
        let mut response_text = String::new();

        loop {
            match response.read(&mut buf[offset..]) {
                Ok(size) => {
                    if size == 0 {
                        break;
                    }

                    total += size;
                    let size_plus_offset = size + offset;

                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            response_text.push_str(text);
                            offset = 0;
                        }
                        Err(error) => {
                            let valid_up_to = error.valid_up_to();
                            unsafe {
                                response_text
                                    .push_str(str::from_utf8_unchecked(&buf[..valid_up_to]));
                            }
                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                        }
                    }
                }
                Err(e) => {
                    return Err(RequestError::Read(format!("Read error: {:?}", e)));
                }
            }
        }
        debug!("Total bytes received: {}", total);
        Ok(response_text)
    }
}

impl Requester for EspRequester {
    type RequestError = RequestError;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), RequestError> {
        let api_key = self.api_key.clone();

        let url = url.to_string();

        let tx = tx.clone();

        thread::spawn(move || {
            // Get a new client
            let mut client = create_client()?;

            // Prepare headers with auth token
            let headers = [
                ("accept", "application/x-ndjson"),
                ("Authorization", &format!("Bearer {}", api_key)),
            ];

            // Create the request
            let request = match client.request(Method::Get, &url, &headers) {
                Ok(req) => req,
                Err(e) => {
                    return Err(RequestError::EspIO(e));
                }
            };

            // Submit the request
            let response = match request.submit() {
                Ok(resp) => resp,
                Err(e) => {
                    return Err(RequestError::EspIO(e));
                }
            };

            let status = response.status();
            debug!("Stream response code: {}", status);

            if !(200..=299).contains(&status) {
                return Err(RequestError::Status(status));
            }

            // Process the streaming response
            let mut buf = [0_u8; 256];
            let mut offset = 0;
            let mut reader = response;

            loop {
                match reader.read(&mut buf[offset..]) {
                    Ok(size) => {
                        if size == 0 {
                            break;
                        }

                        let size_plus_offset = size + offset;

                        match str::from_utf8(&buf[..size_plus_offset]) {
                            Ok(text) => {
                                for line in text.lines() {
                                    if !line.is_empty() && tx.send(line.to_string()).is_err() {
                                        break; // Return if receiver is closed
                                    }
                                }
                                offset = 0;
                            }
                            Err(error) => {
                                let valid_up_to = error.valid_up_to();
                                if valid_up_to > 0 {
                                    unsafe {
                                        let text = str::from_utf8_unchecked(&buf[..valid_up_to]);
                                        for line in text.lines() {
                                            if !line.is_empty()
                                                && tx.send(line.to_string()).is_err()
                                            {
                                                break; // Return if receiver is closed
                                            }
                                        }
                                    }
                                }
                                buf.copy_within(valid_up_to.., 0);
                                offset = size_plus_offset - valid_up_to;
                            }
                        }
                    }
                    Err(e) => {
                        tx.send(format!("Error: {:?}", e)).unwrap();
                        break;
                    }
                }
            }

            Ok(())
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, RequestError> {
        // Get a new client
        let mut client = create_client()?;

        // Prepare headers with auth token
        let headers = [
            ("Content-Type", "application/json"),
            ("accept", "application/json"),
            ("Authorization", &format!("Bearer {}", self.api_key)),
        ];

        // Create the request
        let mut request = match client.request(Method::Post, url, &headers) {
            Ok(req) => req,
            Err(e) => {
                return Err(RequestError::EspIO(e));
            }
        };

        // Add the body data
        if let Err(e) = request.write_all(body.as_bytes()) {
            return Err(RequestError::EspIO(e));
        }

        // Submit the request
        let response = match request.submit() {
            Ok(resp) => resp,
            Err(e) => {
                return Err(RequestError::EspIO(e));
            }
        };

        let status = response.status();
        debug!("POST response code: {}", status);

        // Process the response
        Self::process_response(response, status)
    }
}
