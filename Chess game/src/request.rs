use reqwest;
use std::{sync::mpsc::Sender, thread};

use crate::requester::{RequestError, Requester};

pub struct Request {
    pub api_key: String,
}

impl Requester for Request {
    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), RequestError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(None)
            .build()
            .map_err(|e| {
                RequestError::from(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })?;

        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .map_err(|e| {
                RequestError::from(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })?;

        let mut stream = response.text_stream();
        while let Some(item) = stream.try_next().await {
            match item {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        for line in text.lines() {
                            if !line.is_empty() {
                                tx.send(line.to_string()).unwrap();
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    fn post(&self, url: &str, body: &str) -> Result<String, RequestError> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        
        rt.block_on(async {
            let response = client
                .post(url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .body(body.to_string())
                .send()
                .await
                .map_err(|e| {
                    RequestError::from(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                })?;

            response.text().await.map_err(|e| {
                RequestError::from(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
        })
    }
}
