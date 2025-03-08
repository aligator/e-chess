#![cfg(feature = "reqwest")]

use futures_util::StreamExt;
use reqwest;
use std::sync::mpsc::Sender;

use crate::lichess::LichessConnector;
use crate::requester::{RequestError, Requester};

pub struct Request {
    pub api_key: String,
}

impl Requester for Request {
    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), RequestError> {
        let tx = tx.clone();
        let api_key = self.api_key.clone();
        let url = url.to_string();

        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let response = client
                .get(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
                .unwrap();

            let mut stream = response.bytes_stream();

            while let Some(item) = stream.next().await {
                if let Ok(bytes) = item {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        for line in text.lines() {
                            if !line.is_empty() && tx.send(line.to_string()).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, RequestError> {
        println!("DEBUG: Making POST request to {}", url);
        // Using a channel to get the result from the async operation
        let (tx, rx) = std::sync::mpsc::channel();
        let api_key = self.api_key.clone();
        let url = url.to_string();
        let body = body.to_string();

        println!("DEBUG: Request body: {}", body);

        // Spawn the async operation in the existing runtime
        tokio::spawn(async move {
            println!("DEBUG: Sending request to {} with authorization", url);
            let client = reqwest::Client::new();
            let result = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .body(body)
                .send()
                .await;

            match result {
                Ok(response) => {
                    println!(
                        "DEBUG: Received response with status: {}",
                        response.status()
                    );
                    match response.text().await {
                        Ok(text) => {
                            println!("DEBUG: Received response text: {}", text);
                            let _ = tx.send(Ok(text));
                        }
                        Err(e) => {
                            println!("DEBUG: Error parsing response text: {}", e);
                            let _ = tx.send(Err(RequestError::from(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                            )));
                        }
                    }
                }
                Err(e) => {
                    println!("DEBUG: Error sending request: {}", e);
                    let _ = tx.send(Err(RequestError::from(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )));
                }
            }
        });

        println!("DEBUG: Waiting for response");
        // Wait for the response
        let result = rx
            .recv()
            .unwrap_or(Err(RequestError::from(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Request timeout",
            ))
                as Box<dyn std::error::Error + Send + Sync>)));

        match &result {
            Ok(_) => println!("DEBUG: Received successful response"),
            Err(e) => println!("DEBUG: Request failed with error: {}", e),
        }

        result
    }
}

// Factory functions to create connectors
pub fn create_lichess_connector(api_key: String) -> LichessConnector<Request> {
    LichessConnector::new(Request { api_key })
}
