use crate::bus::MessageBus;
use crate::cli::RunConfig;
use crate::error::AppError;
use crate::message::{BusMessage, ConnectionState, LinePayload, MessageSource};
use crate::parser;
use crate::serial::{self, LineFramer};
use std::cmp::min;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

const RETRY_SLEEP_SLICE_MS: u64 = 100;

enum RuntimeCommand {
    Send { payload: Vec<u8> },
}

/// Queue handle for runtime serial writes.
#[derive(Clone, Debug)]
pub struct RuntimeCommandHandle {
    sender: Sender<RuntimeCommand>,
}

impl RuntimeCommandHandle {
    /// Queues payload bytes to be written by the runtime thread.
    ///
    /// - `payload`: payload bytes, with or without trailing newline.
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
}

/// Cloneable stop handle for future UI or signal integration.
#[derive(Clone, Debug, Default)]
pub struct StopSignal {
    requested: Arc<AtomicBool>,
}

impl StopSignal {
    /// Requests the running app to stop.
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::SeqCst);
    }

    /// Returns whether stop has been requested.
    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

/// Minimal reconnect controller for serial runtime loops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconnectController {
    attempt: u32,
    retry_delay: Duration,
}

impl ReconnectController {
    /// Creates a new reconnect controller.
    ///
    /// - `retry_delay`: delay used before reconnecting.
    pub fn new(retry_delay: Duration) -> Self {
        Self {
            attempt: 0,
            retry_delay,
        }
    }

    /// Starts a new connect attempt and returns its ordinal number.
    pub fn start_attempt(&mut self) -> u32 {
        self.attempt += 1;
        self.attempt
    }

    /// Resets the attempt counter after a successful connection.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Returns the configured retry delay in milliseconds.
    pub fn retry_delay_ms(&self) -> u64 {
        self.retry_delay.as_millis() as u64
    }
}

/// Runtime app that supervises serial connection lifecycle.
pub struct App {
    config: RunConfig,
    bus: MessageBus,
    stop_signal: StopSignal,
    command_rx: Receiver<RuntimeCommand>,
    command_tx: Sender<RuntimeCommand>,
}

impl App {
    /// Creates a new runtime app.
    ///
    /// - `config`: serial runtime configuration.
    /// - `bus`: broadcast bus for downstream consumers.
    pub fn new(config: RunConfig, bus: MessageBus) -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        Self {
            config,
            bus,
            stop_signal: StopSignal::default(),
            command_rx,
            command_tx,
        }
    }

    /// Returns a stop handle that can terminate the run loop.
    pub fn stop_signal(&self) -> StopSignal {
        self.stop_signal.clone()
    }

    /// Returns a command handle that can queue outbound payloads.
    pub fn command_handle(&self) -> RuntimeCommandHandle {
        RuntimeCommandHandle {
            sender: self.command_tx.clone(),
        }
    }

    /// Runs the reconnecting serial ingestion loop.
    pub fn run(&self) -> Result<(), AppError> {
        let source = MessageSource::serial(self.config.port.clone(), self.config.baud_rate);
        self.bus.publish(BusMessage::connection(
            &source,
            ConnectionState::Idle,
            None,
            0,
            None,
        ));

        let mut reconnect = ReconnectController::new(self.config.retry_delay);

        loop {
            if self.stop_signal.is_requested() {
                self.publish_stopped(&source, 0, Some("stop requested".to_string()));
                return Ok(());
            }

            let attempt = reconnect.start_attempt();
            self.bus.publish(BusMessage::connection(
                &source,
                ConnectionState::Connecting,
                None,
                attempt,
                None,
            ));

            match serial::open_port(&self.config) {
                Ok(mut port) => {
                    reconnect.reset();
                    self.bus.publish(BusMessage::connection(
                        &source,
                        ConnectionState::Connected,
                        None,
                        attempt,
                        None,
                    ));

                    let mut framer = LineFramer::with_max_buffer(self.config.max_frame_bytes);

                    loop {
                        if self.stop_signal.is_requested() {
                            self.publish_stopped(
                                &source,
                                attempt,
                                Some("stop requested".to_string()),
                            );
                            return Ok(());
                        }

                        if let Err(error) = self.drain_commands(&source, &mut *port) {
                            self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                            break;
                        }

                        if let Err(error) = serial::pump_port(&mut *port, &mut framer, |line| {
                            let parser = parser::parse_framed_line(self.config.parser, &line);
                            self.bus.publish(
                                BusMessage::rx_line(&source, line.payload).with_parser(parser),
                            );
                        }) {
                            self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                            break;
                        }
                    }
                }
                Err(error) => {
                    self.publish_retry(&source, attempt, error.to_string(), &reconnect);
                }
            }

            if !sleep_with_stop(&self.stop_signal, self.config.retry_delay) {
                self.publish_stopped(&source, attempt, Some("stop requested".to_string()));
                return Ok(());
            }
        }
    }

    fn publish_retry(
        &self,
        source: &MessageSource,
        attempt: u32,
        reason: String,
        reconnect: &ReconnectController,
    ) {
        self.bus.publish(BusMessage::connection(
            source,
            ConnectionState::WaitingRetry,
            Some(reason),
            attempt,
            Some(reconnect.retry_delay_ms()),
        ));
    }

    fn publish_stopped(&self, source: &MessageSource, attempt: u32, reason: Option<String>) {
        self.bus.publish(BusMessage::connection(
            source,
            ConnectionState::Stopped,
            reason,
            attempt,
            None,
        ));
    }

    fn drain_commands<T>(&self, source: &MessageSource, port: &mut T) -> Result<(), AppError>
    where
        T: std::io::Write + ?Sized,
    {
        loop {
            match self.command_rx.try_recv() {
                Ok(RuntimeCommand::Send { payload }) => {
                    serial::write_payload(port, &payload)?;
                    self.bus
                        .publish(BusMessage::tx_line(source, outbound_payload(&payload)));
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
    }
}

fn outbound_payload(payload: &[u8]) -> LinePayload {
    let mut raw = payload.to_vec();

    while matches!(raw.last(), Some(b'\n' | b'\r')) {
        raw.pop();
    }

    let text = String::from_utf8_lossy(&raw).into_owned();
    LinePayload { text, raw }
}

fn sleep_with_stop(stop_signal: &StopSignal, total_delay: Duration) -> bool {
    let mut remaining_ms = total_delay.as_millis() as u64;

    while remaining_ms > 0 {
        if stop_signal.is_requested() {
            return false;
        }

        let sleep_ms = min(remaining_ms, RETRY_SLEEP_SLICE_MS);
        thread::sleep(Duration::from_millis(sleep_ms));
        remaining_ms -= sleep_ms;
    }

    true
}
