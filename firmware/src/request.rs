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
use std::fmt;
use std::sync::mpsc::Sender;
use std::thread::{self};
use std::{error::Error, fmt::Debug};

#[derive(Debug)]
pub enum RequestError {
    Esp(EspError),
    EspIO(EspIOError),
    Status(u16),
    Read(String),
}

impl Error for RequestError {}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::Esp(e) => write!(f, "ESP error: {:?}", e),
            RequestError::EspIO(e) => write!(f, "ESP IO error: {:?}", e),
            RequestError::Status(code) => write!(f, "HTTP status error: {}", code),
            RequestError::Read(msg) => write!(f, "Read error: {}", msg),
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
pub struct EspRequester {
    api_key: String,
}

impl Debug for EspRequester {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EspRequester {{ api_key: {:?} }}", self.api_key)
    }
}

impl EspRequester {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    /// Helper function to read a chunk of data from a Read source and convert it to UTF-8
    /// Returns a tuple containing:
    /// - The number of bytes read
    /// - The valid UTF-8 string from the current read
    /// - The number of bytes that should be kept as offset for the next read
    fn read_utf8_chunk(
        response: &mut impl Read,
        buf: &mut [u8],
        offset: usize,
    ) -> Result<(usize, String, usize), RequestError> {
        // Read into the buffer starting at the offset
        debug!("Reading ...");
        let bytes_read = match response.read(&mut buf[offset..]) {
            Ok(size) => {
                if size == 0 {
                    return Ok((0, String::new(), offset));
                }
                debug!("Receiving Chunks of data");

                size
            }
            Err(e) => {
                info!("Error reading response: {:?}", e);
                return Err(RequestError::Read(format!("Read error: {:?}", e)));
            }
        };

        let total_size = bytes_read + offset;

        debug!("Parse utf8");
        // Try to convert the entire buffer to a UTF-8 string
        match str::from_utf8(&buf[..total_size]) {
            Ok(s) => {
                // All data is valid UTF-8
                debug!("Parsing done");
                Ok((bytes_read, s.to_string(), 0))
            }
            Err(e) => {
                // Only part of the data is valid UTF-8
                let valid_up_to = e.valid_up_to();

                // Extract the valid part
                let valid_str = if valid_up_to > 0 {
                    // Safe because we've verified these bytes are valid UTF-8
                    unsafe { str::from_utf8_unchecked(&buf[..valid_up_to]) }.to_string()
                } else {
                    String::new()
                };

                // Move the remaining bytes to the beginning of the buffer
                if valid_up_to < total_size {
                    let remaining = total_size - valid_up_to;
                    buf.copy_within(valid_up_to..total_size, 0);
                    Ok((bytes_read, valid_str, remaining))
                } else {
                    Ok((bytes_read, valid_str, 0))
                }
            }
        }
    }

    // Helper to process HTTP response to string
    fn process_response(mut response: impl Read, status: u16) -> Result<String, RequestError> {
        if !(200..=299).contains(&status) {
            info!("Response failed with status: {}", status);
            return Err(RequestError::Status(status));
        }

        let mut buf = [0_u8; 256];
        let mut offset = 0;
        let mut response_text = String::new();

        loop {
            match EspRequester::read_utf8_chunk(&mut response, &mut buf, offset) {
                Ok((size, text, new_offset)) => {
                    if size == 0 {
                        info!("End of response reached (zero bytes)");
                        break;
                    }

                    response_text.push_str(&text);
                    offset = new_offset;
                }
                Err(e) => {
                    info!("Error reading response: {:?}", e);
                    return Err(e);
                }
            }
        }
        Ok(response_text)
    }
}

impl Requester for EspRequester {
    type RequestError = RequestError;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), RequestError> {
        info!("Starting stream request to: {}", url);
        let api_key = self.api_key.clone();
        let url = url.to_string();
        let tx = tx.clone();

        thread::spawn(move || {
            // Get a new client
            let mut client = match create_client() {
                Ok(client) => client,
                Err(e) => {
                    error!("Failed to create HTTP client: {:?}", e);
                    return Err(e);
                }
            };

            // Prepare headers with auth token
            let headers = [
                ("accept", "application/x-ndjson"),
                ("Authorization", &format!("Bearer {}", api_key)),
            ];

            // Create the request
            let request = match client.request(Method::Get, &url, &headers) {
                Ok(req) => req,
                Err(e) => {
                    error!("Error creating stream request: {:?}", e);
                    return Err(RequestError::EspIO(e));
                }
            };

            // Submit the request
            let mut response = match request.submit() {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Error submitting stream request: {:?}", e);
                    return Err(RequestError::EspIO(e));
                }
            };

            let status = response.status();

            if !(200..=299).contains(&status) {
                error!("Stream request failed with status: {}", status);
                return Err(RequestError::Status(status));
            }

            // Process the streaming response using the read_utf8_chunk helper
            let mut buf = [0_u8; 128]; // Buffer for reading
            let mut offset = 0;
            let mut accumulated_data = String::new();

            loop {
                match EspRequester::read_utf8_chunk(&mut response, &mut buf, offset) {
                    Ok((size, text, new_offset)) => {
                        if size == 0 {
                            // Process any remaining accumulated data
                            if !accumulated_data.is_empty() {
                                info!("Event received");
                                match tx.send(accumulated_data) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        warn!("Failed to send on channel (likely closed): {:?}", e);
                                        return Ok(());
                                    }
                                }
                            }

                            break;
                        }

                        if text.trim().is_empty() {
                            continue;
                        }

                        // Append the new text to our accumulated data
                        accumulated_data.push_str(&text);
                        // Process complete lines
                        if accumulated_data.contains('\n') {
                            let lines: Vec<&str> = accumulated_data.split('\n').collect();

                            // Process all complete lines except the last one (which might be incomplete)
                            for i in 0..lines.len() - 1 {
                                let line = lines[i];
                                if !line.is_empty() {
                                    match tx.send(line.to_string()) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            warn!(
                                                "Failed to send on channel (likely closed): {:?}",
                                                e
                                            );
                                            return Ok(());
                                        }
                                    }
                                }
                            }

                            // Keep the last line which might be incomplete
                            accumulated_data = lines.last().unwrap().to_string();
                        }

                        offset = new_offset;
                    }
                    Err(e) => {
                        error!("Error reading from stream: {:?}", e);
                        match tx.send(format!("Error: {:?}", e)) {
                            Ok(_) => {}
                            Err(send_err) => {
                                warn!("Failed to send error on channel: {:?}", send_err)
                            }
                        }
                        break;
                    }
                }
            }
            Ok(())
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, RequestError> {
        info!("Starting POST request to: {}", url);

        // Prepare headers with auth token
        let headers = [
            ("Content-Type", "application/json"),
            ("accept", "application/json"),
            ("Authorization", &format!("Bearer {}", self.api_key)),
        ];

        // Create the request
        // It is more stable to create a new client each time. But maybe not fast...
        let mut client = create_client()?;
        let mut request = match client.request(Method::Post, url, &headers) {
            Ok(req) => req,
            Err(e) => {
                info!("Error creating POST request: {:?}", e);
                return Err(RequestError::EspIO(e));
            }
        };

        // Add the body data
        if let Err(e) = request.write_all(body.as_bytes()) {
            info!("Error writing POST request body: {:?}", e);
            return Err(RequestError::EspIO(e));
        }

        // Submit the request
        let response = match request.submit() {
            Ok(resp) => resp,
            Err(e) => {
                info!("Error submitting POST request: {:?}", e);
                return Err(RequestError::EspIO(e));
            }
        };

        let status = response.status();

        // Process the response
        let result = EspRequester::process_response(response, status);
        match &result {
            Ok(response_text) => {
                info!("POST request completed successfully");
                debug!("POST response body: {}", response_text);
            }
            Err(e) => {
                info!("POST request failed: {:?}", e);
            }
        }
        result
    }
}
