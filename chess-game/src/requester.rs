use std::{fmt::Debug, sync::mpsc::Sender};

/// Trait for sending and receiving requests.
/// Abstracts away the details of the request implementation.
/// This makes it more easy to switch out the implementation
/// on the micro controller as it is not the same as in this
/// example application.
///
/// It can be used by the chess connectors.
pub trait Requester {
    type RequestError: Debug + std::error::Error;

    fn stream(&self, tx: &mut Sender<String>, url: &str) -> Result<(), Self::RequestError>;
    fn post(&self, url: &str, body: &str) -> Result<String, Self::RequestError>;
    fn get(&self, url: &str) -> Result<String, Self::RequestError>;

    fn is_connected(&self) -> bool;
}

#[derive(Debug)]
pub struct DummyError;

impl std::fmt::Display for DummyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dummy error")
    }
}

impl std::error::Error for DummyError {}

#[derive(Debug)]
pub struct DummyRequester;

impl Requester for DummyRequester {
    type RequestError = DummyError;

    fn stream(&self, _tx: &mut Sender<String>, _url: &str) -> Result<(), Self::RequestError> {
        Ok(())
    }

    fn post(&self, _url: &str, _body: &str) -> Result<String, Self::RequestError> {
        Ok(String::new())
    }

    fn get(&self, _url: &str) -> Result<String, Self::RequestError> {
        Ok(String::new())
    }

    fn is_connected(&self) -> bool {
        true
    }
}
