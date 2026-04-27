use crate::ai_adapter::AiContextAdapter;
use crate::ai_contract::AiSessionSnapshot;
use crate::app::{App, RuntimeCommandHandle, StopSignal};
use crate::bmi088::Bmi088HostCommand;
use crate::bus::{BusSubscription, MessageBus};
use crate::cli::RunConfig;
use crate::error::AppError;
use crate::fake_session::{self, FakeSessionHandle};
use crate::serial::{self, SerialDeviceInfo};
use std::io::ErrorKind;
use std::sync::{Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

/// Runtime source configuration for one shared telemetry session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeSessionConfig {
    Serial(RunConfig),
    Fake(FakeRuntimeConfig),
}

/// Fake runtime source options shared by desktop and MCP flows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeRuntimeConfig {
    pub profile: String,
    pub baud_rate: u32,
}

/// Session-scoped runtime host that keeps one live bus and one AI adapter in sync.
pub struct SessionRuntimeHost {
    adapter: AiContextAdapter,
    inner: Mutex<RuntimeState>,
}

#[derive(Default)]
struct RuntimeState {
    running: Option<RunningSession>,
}

struct RunningSession {
    bus: MessageBus,
    control: RuntimeControl,
    worker: JoinHandle<()>,
    adapter_worker: JoinHandle<()>,
}

enum RuntimeControl {
    Serial {
        stop_signal: StopSignal,
        command_handle: RuntimeCommandHandle,
    },
    Fake(FakeSessionHandle),
}

impl Default for SessionRuntimeHost {
    fn default() -> Self {
        Self::new(AiContextAdapter::default())
    }
}

impl SessionRuntimeHost {
    /// Creates a host around one stable AI adapter instance.
    pub fn new(adapter: AiContextAdapter) -> Self {
        Self {
            adapter,
            inner: Mutex::new(RuntimeState::default()),
        }
    }

    /// Returns the stable AI adapter mirrored from the currently running session.
    pub fn adapter(&self) -> AiContextAdapter {
        self.adapter.clone()
    }

    /// Returns the current AI session snapshot.
    pub fn session_snapshot(&self) -> AiSessionSnapshot {
        self.adapter.session_snapshot()
    }

    /// Starts a new telemetry session and returns a subscription for UI forwarding.
    pub fn connect(&self, config: RuntimeSessionConfig) -> Result<BusSubscription, AppError> {
        let mut state = lock_state(&self.inner);
        if state.running.is_some() {
            return Err(AppError::Io(std::io::Error::new(
                ErrorKind::AlreadyExists,
                "runtime session is already running",
            )));
        }

        self.adapter.reset();

        let bus = MessageBus::new();
        let ui_subscription = bus.subscribe();
        let adapter_worker = self.adapter.spawn(bus.subscribe());
        let (control, worker) = spawn_session(config, bus.clone());

        state.running = Some(RunningSession {
            bus,
            control,
            worker,
            adapter_worker,
        });

        Ok(ui_subscription)
    }

    /// Stops the current session and clears mirrored AI snapshots.
    pub fn disconnect(&self) -> Result<(), AppError> {
        let running = {
            let mut state = lock_state(&self.inner);
            state.running.take().ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    ErrorKind::NotConnected,
                    "runtime session is not running",
                ))
            })?
        };

        let RunningSession {
            bus,
            control,
            worker,
            adapter_worker,
        } = running;

        control.request_stop();
        let _ = worker.join();
        drop(bus);
        let _ = adapter_worker.join();
        self.adapter.reset();
        Ok(())
    }

    /// Sends one outbound payload through the active runtime session.
    pub fn send_payload(&self, payload: Vec<u8>) -> Result<(), AppError> {
        let snapshot = self.adapter.session_snapshot();
        let connection = snapshot.connection.ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not connected",
            ))
        })?;

        if connection.state != crate::message::ConnectionState::Connected {
            return Err(AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not connected",
            )));
        }

        let state = lock_state(&self.inner);
        let running = state.running.as_ref().ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not running",
            ))
        })?;

        running.control.send_payload(payload)
    }

    /// Sends one BMI088 host command through the active runtime session.
    pub fn send_bmi088_command(
        &self,
        command: Bmi088HostCommand,
        payload: Option<Vec<u8>>,
    ) -> Result<(), AppError> {
        let snapshot = self.adapter.session_snapshot();
        let connection = snapshot.connection.ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not connected",
            ))
        })?;

        if connection.state != crate::message::ConnectionState::Connected {
            return Err(AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not connected",
            )));
        }

        let state = lock_state(&self.inner);
        let running = state.running.as_ref().ok_or_else(|| {
            AppError::Io(std::io::Error::new(
                ErrorKind::NotConnected,
                "runtime session is not running",
            ))
        })?;

        running.control.send_bmi088_command(command, payload)
    }

    /// Lists serial ports using the shared backend helper.
    pub fn list_ports(&self) -> Result<Vec<String>, AppError> {
        serial::list_ports()
    }

    /// Lists UI-visible serial devices with host metadata.
    pub fn list_visible_devices(&self) -> Result<Vec<SerialDeviceInfo>, AppError> {
        serial::list_visible_devices()
    }
}

impl RuntimeControl {
    fn request_stop(&self) {
        match self {
            Self::Serial { stop_signal, .. } => stop_signal.request_stop(),
            Self::Fake(handle) => handle.request_stop(),
        }
    }

    fn send_payload(&self, payload: Vec<u8>) -> Result<(), AppError> {
        match self {
            Self::Serial { command_handle, .. } => command_handle.send_payload(payload),
            Self::Fake(handle) => handle
                .send_payload(payload)
                .map_err(|error| AppError::Io(std::io::Error::new(ErrorKind::BrokenPipe, error))),
        }
    }

    fn send_bmi088_command(
        &self,
        command: Bmi088HostCommand,
        payload: Option<Vec<u8>>,
    ) -> Result<(), AppError> {
        match self {
            Self::Serial { command_handle, .. } => command_handle.send_bmi088_command(command, payload),
            Self::Fake(handle) => handle
                .send_payload(match payload.as_deref() {
                    Some(payload) => crate::bmi088::encode_host_command_with_payload(command, payload),
                    None => crate::bmi088::encode_host_command(command),
                })
                .map_err(|error| AppError::Io(std::io::Error::new(ErrorKind::BrokenPipe, error))),
        }
    }
}

fn spawn_session(
    config: RuntimeSessionConfig,
    bus: MessageBus,
) -> (RuntimeControl, JoinHandle<()>) {
    match config {
        RuntimeSessionConfig::Serial(config) => {
            let app = App::new(config, bus);
            let stop_signal = app.stop_signal();
            let command_handle = app.command_handle();
            let worker = thread::spawn(move || {
                let _ = app.run();
            });

            (
                RuntimeControl::Serial {
                    stop_signal,
                    command_handle,
                },
                worker,
            )
        }
        RuntimeSessionConfig::Fake(config) => {
            let port = fake_session::fake_port_label(&config.profile);
            let (handle, worker) = fake_session::spawn(
                fake_session::FakeSessionConfig {
                    port,
                    baud_rate: config.baud_rate,
                    profile: config.profile,
                },
                bus,
            );

            (RuntimeControl::Fake(handle), worker)
        }
    }
}

fn lock_state<'a>(state: &'a Mutex<RuntimeState>) -> MutexGuard<'a, RuntimeState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
