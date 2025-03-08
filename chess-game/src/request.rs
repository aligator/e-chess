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
        // Using a channel to get the result from the async operation
        let (tx, rx) = std::sync::mpsc::channel();
        let api_key = self.api_key.clone();
        let url = url.to_string();
        let body = body.to_string();

        // Spawn the async operation in the existing runtime
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let result = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .body(body)
                .send()
                .await;

            match result {
                Ok(response) => match response.text().await {
                    Ok(text) => {
                        let _ = tx.send(Ok(text));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(RequestError::from(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                        )));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(RequestError::from(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )));
                }
            }
        });

        // Wait for the response
        let result = rx
            .recv()
            .unwrap_or(Err(RequestError::from(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Request timeout",
            ))
                as Box<dyn std::error::Error + Send + Sync>)));

        result
    }
}

// Factory functions to create connectors
pub fn create_lichess_connector(api_key: String) -> LichessConnector<Request> {
    LichessConnector::new(Request { api_key })
}
