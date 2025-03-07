use core::fmt;
use std::sync::mpsc::Sender;

use thiserror::Error;

#[derive(Debug, Error)]
pub struct RequestError {
    #[from]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}

/// Trait for sending and receiving requests.
/// Abstracts away the details of the request implementation.
/// This makes it more easy to switch out the implementation
/// on the micro controller as it is not the same as in this
/// example application.
///
/// It can be used by the chess connectors.
pub trait Requester {
    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), RequestError>;
    fn post(&self, url: &str, body: &str) -> Result<String, RequestError>;
}
