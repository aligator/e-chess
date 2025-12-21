#![cfg(feature = "reqwest")]

use futures_util::StreamExt;
use reqwest;
use std::sync::mpsc::{RecvError, Sender};
use thiserror::Error;

use crate::requester::Requester;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error(transparent)]
    Request(reqwest::Error),
    #[error(transparent)]
    Recv(RecvError),
}

#[derive(Debug)]
pub struct Request {
    pub api_key: String,
}

impl Request {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl Requester for Request {
    type RequestError = RequestError;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), self::RequestError> {
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

    fn get(&self, url: &str) -> Result<String, self::RequestError> {
        // Using a channel to get the result from the async operation
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, RequestError>>();
        let api_key = self.api_key.clone();
        let url = url.to_string();

        // Spawn the async operation in the existing runtime
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let result = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await;

            match result {
                Ok(response) => match response.text().await {
                    Ok(text) => {
                        let _ = tx.send(Ok(text));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(RequestError::Request(e)));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(RequestError::Request(e)));
                }
            }
        });

        // Wait for the response
        rx.recv().map_err(|e| RequestError::Recv(e))?
    }

    fn post(&self, url: &str, body: &str) -> Result<String, self::RequestError> {
        // Using a channel to get the result from the async operation
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, RequestError>>();
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
                        let _ = tx.send(Err(RequestError::Request(e)));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(RequestError::Request(e)));
                }
            }
        });

        // Wait for the response
        rx.recv().map_err(|e| RequestError::Recv(e))?
    }
}
