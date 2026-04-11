use crate::model::{
    ConnectRequest, SendRequest, SessionSnapshot, SourceKind, UiBusEvent, UiConnectionState,
    UiTransportKind, apply_connection_snapshot,
};
use crate::{fake_session, fake_session::FakeSessionHandle};
use eadai::app::{App, RuntimeCommandHandle, StopSignal};
use eadai::bus::BusSubscription;
use eadai::cli::{ParserKind, RunConfig};
use eadai::serial;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const SERIAL_EVENT_NAME: &str = "serial-bus-event";

#[derive(Default)]
pub struct DesktopState {
    inner: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    running: Option<RunningSession>,
    last_snapshot: SessionSnapshot,
}

struct RunningSession {
    control: SessionControl,
    snapshot: Arc<Mutex<SessionSnapshot>>,
    worker: JoinHandle<()>,
    forwarder: JoinHandle<()>,
}

enum SessionControl {
    Real {
        stop_signal: StopSignal,
        command_handle: RuntimeCommandHandle,
    },
    Fake(FakeSessionHandle),
}

impl SessionControl {
    fn request_stop(&self) {
        match self {
            Self::Real { stop_signal, .. } => stop_signal.request_stop(),
            Self::Fake(handle) => handle.request_stop(),
        }
    }

    fn send_payload(&self, payload: Vec<u8>) -> Result<(), String> {
        match self {
            Self::Real { command_handle, .. } => command_handle
                .send_payload(payload)
                .map_err(|error| error.to_string()),
            Self::Fake(handle) => handle.send_payload(payload),
        }
    }
}

impl DesktopState {
    pub fn list_ports(&self) -> Result<Vec<String>, String> {
        serial::list_ports().map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let state = lock_state(&self.inner);

        match &state.running {
            Some(session) => lock_snapshot(&session.snapshot).clone(),
            None => state.last_snapshot.clone(),
        }
    }

    pub fn connect(
        &self,
        app_handle: &AppHandle,
        request: ConnectRequest,
    ) -> Result<SessionSnapshot, String> {
        let mut state = lock_state(&self.inner);
        if state.running.is_some() {
            return Err("serial session is already running".to_string());
        }

        let bus = eadai::bus::MessageBus::new();
        let subscription = bus.subscribe();
        let snapshot = Arc::new(Mutex::new(initial_snapshot(&request)));
        let (control, worker) = spawn_session(request, bus);
        let forwarder = spawn_forwarder(app_handle.clone(), subscription, Arc::clone(&snapshot));
        let current_snapshot = lock_snapshot(&snapshot).clone();

        state.last_snapshot = current_snapshot.clone();
        state.running = Some(RunningSession {
            control,
            snapshot,
            worker,
            forwarder,
        });

        Ok(current_snapshot)
    }

    pub fn disconnect(&self) -> Result<SessionSnapshot, String> {
        let running = {
            let mut state = lock_state(&self.inner);
            state
                .running
                .take()
                .ok_or_else(|| "serial session is not running".to_string())?
        };

        running.control.request_stop();
        let _ = running.worker.join();
        let _ = running.forwarder.join();

        let mut snapshot = lock_snapshot(&running.snapshot).clone();
        snapshot.is_running = false;
        snapshot.connection_state = Some(UiConnectionState::Stopped);

        let mut state = lock_state(&self.inner);
        state.last_snapshot = snapshot.clone();
        Ok(snapshot)
    }

    pub fn send(&self, request: SendRequest) -> Result<(), String> {
        let state = lock_state(&self.inner);
        let session = state
            .running
            .as_ref()
            .ok_or_else(|| "serial session is not running".to_string())?;
        let snapshot = lock_snapshot(&session.snapshot).clone();

        if snapshot.connection_state != Some(UiConnectionState::Connected) {
            return Err("serial session is not connected".to_string());
        }

        session.control.send_payload(serial::payload_bytes_for_text(
            &request.payload,
            request.append_newline,
        ))
    }
}

fn initial_snapshot(request: &ConnectRequest) -> SessionSnapshot {
    match request.source_kind {
        SourceKind::Serial => SessionSnapshot::connecting(
            UiTransportKind::Serial,
            request.port.clone(),
            request.baud_rate,
        ),
        SourceKind::Fake => SessionSnapshot::connecting(
            UiTransportKind::Fake,
            fake_session::fake_port_label(
                request
                    .fake_profile
                    .as_deref()
                    .unwrap_or(fake_session::default_profile()),
            ),
            request.baud_rate,
        ),
    }
}

fn spawn_session(
    request: ConnectRequest,
    bus: eadai::bus::MessageBus,
) -> (SessionControl, JoinHandle<()>) {
    match request.source_kind {
        SourceKind::Serial => {
            let config = RunConfig {
                port: request.port,
                baud_rate: request.baud_rate,
                retry_delay: Duration::from_millis(request.retry_ms),
                read_timeout: Duration::from_millis(request.read_timeout_ms),
                parser: ParserKind::Auto,
                max_frame_bytes: eadai::cli::DEFAULT_MAX_FRAME_BYTES,
            };
            let app = App::new(config, bus);
            let stop_signal = app.stop_signal();
            let command_handle = app.command_handle();
            let worker = thread::spawn(move || {
                let _ = app.run();
            });

            (
                SessionControl::Real {
                    stop_signal,
                    command_handle,
                },
                worker,
            )
        }
        SourceKind::Fake => {
            let profile = request
                .fake_profile
                .unwrap_or_else(|| fake_session::default_profile().to_string());
            let port = fake_session::fake_port_label(&profile);
            let (handle, worker) = fake_session::spawn(
                fake_session::FakeSessionConfig {
                    port,
                    baud_rate: request.baud_rate,
                    profile,
                },
                bus,
            );

            (SessionControl::Fake(handle), worker)
        }
    }
}

fn spawn_forwarder(
    app_handle: AppHandle,
    subscription: BusSubscription,
    snapshot: Arc<Mutex<SessionSnapshot>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(message) = subscription.recv() {
            let event = UiBusEvent::from(message);

            if let UiBusEvent::Connection {
                ref source,
                ref connection,
                ..
            } = event
            {
                apply_connection_snapshot(&mut lock_snapshot(&snapshot), connection, source);
            }

            let _ = app_handle.emit(SERIAL_EVENT_NAME, &event);
        }
    })
}

fn lock_state<'a>(state: &'a Mutex<SessionState>) -> MutexGuard<'a, SessionState> {
    match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lock_snapshot<'a>(snapshot: &'a Arc<Mutex<SessionSnapshot>>) -> MutexGuard<'a, SessionSnapshot> {
    match snapshot.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
