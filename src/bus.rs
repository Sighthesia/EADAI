use crate::message::BusMessage;
use std::sync::mpsc::{self, Receiver, RecvError, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// Fan-out message bus for backend runtime consumers.
#[derive(Clone, Default)]
pub struct MessageBus {
    subscribers: Arc<Mutex<Vec<Sender<BusMessage>>>>,
}

impl MessageBus {
    /// Creates an empty message bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new subscriber.
    pub fn subscribe(&self) -> BusSubscription {
        let (sender, receiver) = mpsc::channel();
        let mut subscribers = lock_subscribers(&self.subscribers);
        subscribers.push(sender);
        BusSubscription { receiver }
    }

    /// Broadcasts one message to all current subscribers.
    ///
    /// - `message`: envelope to broadcast.
    pub fn publish(&self, message: BusMessage) {
        let mut subscribers = lock_subscribers(&self.subscribers);
        subscribers.retain(|sender| sender.send(message.clone()).is_ok());
    }
}

/// Subscription handle for one bus consumer.
pub struct BusSubscription {
    receiver: Receiver<BusMessage>,
}

impl BusSubscription {
    /// Waits until a message arrives or the bus is dropped.
    pub fn recv(&self) -> Result<BusMessage, RecvError> {
        self.receiver.recv()
    }

    /// Waits for a bounded amount of time for a message.
    ///
    /// - `timeout`: maximum wait duration.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<BusMessage, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

fn lock_subscribers<'a>(
    subscribers: &'a Arc<Mutex<Vec<Sender<BusMessage>>>>,
) -> MutexGuard<'a, Vec<Sender<BusMessage>>> {
    match subscribers.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
