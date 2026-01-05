use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// EventManager using queues for event processing
pub struct EventManager<T: Send + Clone + 'static> {
    senders: Arc<Mutex<Vec<Sender<T>>>>,

    // The global receiver for all events
    receiver: Arc<Mutex<Receiver<T>>>,

    // The global sender for all events - Sender is already Clone + Send + Sync, no need for Arc<Mutex<>>
    sender: Sender<T>,
}

impl<T: Send + Clone + 'static> EventManager<T> {
    /// Creates a new EventManager
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        EventManager {
            senders: Arc::new(Mutex::new(Vec::new())),
            receiver: Arc::new(Mutex::new(receiver)),
            sender,
        }
    }

    /// Creates a new sender for a specific event type
    pub fn create_sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    /// Creates a new receiver for a specific event type
    pub fn create_receiver(&self) -> Receiver<T> {
        let (sender, receiver) = channel();
        let mut senders = self.senders.lock().unwrap();
        senders.push(sender);
        receiver
    }

    pub fn start_thread(&self) {
        let receiver = self.receiver.clone();
        let senders = self.senders.clone();
        std::thread::spawn(move || loop {
            let event = receiver.lock().unwrap().recv().unwrap();
            for sender in senders.lock().unwrap().iter() {
                sender.send(event.clone()).unwrap();
            }
        });
    }
}

impl<T: Send + Clone + 'static> Default for EventManager<T> {
    fn default() -> Self {
        Self::new()
    }
}
