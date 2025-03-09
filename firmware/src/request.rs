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
use std::cell::RefCell;
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
    client: RefCell<Client<EspHttpConnection>>,
}

impl Debug for EspRequester {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EspRequester {{ api_key: {:?} }}", self.api_key)
    }
}

impl EspRequester {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: RefCell::new(create_client().unwrap()),
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
        let mut total = 0;
        let mut response_text = String::new();

        info!("Starting to read response body");
        loop {
            match response.read(&mut buf[offset..]) {
                Ok(size) => {
                    if size == 0 {
                        info!("End of response reached (zero bytes)");
                        break;
                    }

                    total += size;
                    let size_plus_offset = size + offset;
                    info!(
                        "Read {} bytes (total: {}, buffer size: {})",
                        size, total, size_plus_offset
                    );

                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            info!("Successfully converted {} bytes to UTF-8", size_plus_offset);
                            response_text.push_str(text);
                            offset = 0;
                        }
                        Err(error) => {
                            let valid_up_to = error.valid_up_to();
                            info!(
                                "Partial UTF-8 conversion: valid up to {} of {} bytes",
                                valid_up_to, size_plus_offset
                            );

                            if valid_up_to > 0 {
                                unsafe {
                                    response_text
                                        .push_str(str::from_utf8_unchecked(&buf[..valid_up_to]));
                                }
                            }

                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                            info!("Remaining bytes in buffer: {}", offset);
                        }
                    }
                }
                Err(e) => {
                    info!("Error reading response: {:?}", e);
                    return Err(RequestError::Read(format!("Read error: {:?}", e)));
                }
            }
        }
        info!(
            "Total bytes received: {}, final response length: {}",
            total,
            response_text.len()
        );
        if response_text.len() > 0 {
            info!(
                "Response preview: {}",
                if response_text.len() > 100 {
                    format!("{}...", &response_text[..100])
                } else {
                    response_text.clone()
                }
            );
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
            info!("Creating HTTP client for stream request");
            let mut client = create_client()?;

            // Prepare headers with auth token
            let headers = [
                ("accept", "application/x-ndjson"),
                ("Authorization", &format!("Bearer {}", api_key)),
            ];

            info!("Headers: {:?}", headers);

            // Create the request
            info!("Sending GET stream request to: {}", url);
            let request = match client.request(Method::Get, &url, &headers) {
                Ok(req) => req,
                Err(e) => {
                    info!("Error creating stream request: {:?}", e);
                    return Err(RequestError::EspIO(e));
                }
            };

            // Submit the request
            let response = match request.submit() {
                Ok(resp) => resp,
                Err(e) => {
                    info!("Error submitting stream request: {:?}", e);
                    return Err(RequestError::EspIO(e));
                }
            };

            let status = response.status();
            info!("Stream response status code: {}", status);

            if !(200..=299).contains(&status) {
                info!("Stream request failed with status: {}", status);
                return Err(RequestError::Status(status));
            }

            // Process the streaming response
            info!("Processing stream response");
            let mut buf = [0_u8; 1024]; // Increased buffer size for more efficient reading
            let mut reader = response;
            let mut total_bytes = 0;
            let mut line_count = 0;

            // Buffer to accumulate partial JSON data
            let mut accumulated_data = String::new();

            loop {
                match reader.read(&mut buf) {
                    Ok(size) => {
                        if size == 0 {
                            info!("Stream ended (zero bytes received). Total received: {} bytes, {} lines", total_bytes, line_count);

                            // Process any remaining accumulated data
                            if !accumulated_data.is_empty() {
                                info!(
                                    "Processing remaining accumulated data ({} chars)",
                                    accumulated_data.len()
                                );
                                if tx.send(accumulated_data).is_err() {
                                    info!("Stream channel closed, stopping stream");
                                }
                                line_count += 1;
                            }

                            break;
                        }

                        total_bytes += size;
                        info!("Read {} bytes from stream (total: {})", size, total_bytes);

                        // Convert bytes to string and handle UTF-8 errors more efficiently
                        match std::str::from_utf8(&buf[..size]) {
                            Ok(text) => {
                                // Append the new text to our accumulated data
                                accumulated_data.push_str(text);
                                info!("Accumulated data size: {} chars", accumulated_data.len());
                            }
                            Err(e) => {
                                // Handle partial UTF-8 sequences more efficiently
                                let valid_up_to = e.valid_up_to();

                                if valid_up_to > 0 {
                                    // Add the valid part
                                    let valid_text = unsafe {
                                        std::str::from_utf8_unchecked(&buf[..valid_up_to])
                                    };
                                    accumulated_data.push_str(valid_text);
                                }

                                // If there's an incomplete UTF-8 sequence at the end, we need to handle it
                                if let Some(incomplete_char) = e.error_len() {
                                    // Copy the incomplete bytes to the beginning of the buffer for the next read
                                    let remainder = &buf[valid_up_to..size];
                                    info!(
                                        "Found incomplete UTF-8 sequence of {} bytes",
                                        remainder.len()
                                    );

                                    // We'll just ignore the incomplete sequence for now as it will be completed
                                    // in the next read. This is a simplification that works for most streaming APIs.
                                }
                            }
                        }

                        // Process complete lines
                        if accumulated_data.contains('\n') {
                            let lines: Vec<&str> = accumulated_data.split('\n').collect();

                            // Process all complete lines except the last one (which might be incomplete)
                            for i in 0..lines.len() - 1 {
                                let line = lines[i];
                                if !line.is_empty() {
                                    info!("Processing complete line ({} chars)", line.len());

                                    if tx.send(line.to_string()).is_err() {
                                        info!("Stream channel closed, stopping stream");
                                        return Ok(());
                                    }
                                    line_count += 1;
                                }
                            }

                            // Keep the last line which might be incomplete
                            accumulated_data = lines.last().unwrap().to_string();
                        }
                    }
                    Err(e) => {
                        info!("Error reading from stream: {:?}", e);
                        tx.send(format!("Error: {:?}", e)).unwrap();
                        break;
                    }
                }
            }
            info!(
                "Stream processing completed. Total processed: {} bytes, {} lines",
                total_bytes, line_count
            );
            Ok(())
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, RequestError> {
        info!("Starting POST request to: {}", url);
        info!("POST request body: {}", body);

        // Prepare headers with auth token
        let headers = [
            ("Content-Type", "application/json"),
            ("accept", "application/json"),
            ("Authorization", &format!("Bearer {}", self.api_key)),
        ];

        // Create the request
        info!("Preparing POST request to: {}", url);
        // let mut client = self.client.borrow_mut();
        // may be more stable to create a new client each time
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
        info!("Submitting POST request");
        let response = match request.submit() {
            Ok(resp) => resp,
            Err(e) => {
                info!("Error submitting POST request: {:?}", e);
                return Err(RequestError::EspIO(e));
            }
        };

        let status = response.status();
        info!("POST response status code: {}", status);

        // Process the response
        info!("Processing POST response");
        let result = Self::process_response(response, status);
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
