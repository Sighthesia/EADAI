use crate::bmi088::Bmi088HostCommand;
use crate::error::AppError;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

pub(super) enum RuntimeCommand {
    Send { payload: Vec<u8> },
    SendBmi088 { command: Bmi088HostCommand, payload: Option<Vec<u8>> },
}

#[derive(Clone, Debug)]
pub struct RuntimeCommandHandle {
    sender: Sender<RuntimeCommand>,
}

impl RuntimeCommandHandle {
    pub(super) fn new(sender: Sender<RuntimeCommand>) -> Self {
        Self { sender }
    }

    pub fn send_payload(&self, payload: Vec<u8>) -> Result<(), AppError> {
        self.sender
            .send(RuntimeCommand::Send { payload })
            .map_err(|_| {
                AppError::Io(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "runtime command channel is closed",
                ))
            })
    }

    pub fn send_bmi088_command(
        &self,
        command: Bmi088HostCommand,
        payload: Option<Vec<u8>>,
    ) -> Result<(), AppError> {
        self.sender
            .send(RuntimeCommand::SendBmi088 { command, payload })
            .map_err(|_| {
                AppError::Io(std::io::Error::new(
                    ErrorKind::BrokenPipe,
                    "runtime command channel is closed",
                ))
            })
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopSignal {
    requested: Arc<AtomicBool>,
}

impl StopSignal {
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconnectController {
    attempt: u32,
    retry_delay: Duration,
}

impl ReconnectController {
    pub fn new(retry_delay: Duration) -> Self {
        Self {
            attempt: 0,
            retry_delay,
        }
    }

    pub fn start_attempt(&mut self) -> u32 {
        self.attempt += 1;
        self.attempt
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn retry_delay_ms(&self) -> u64 {
        self.retry_delay.as_millis() as u64
    }
}
