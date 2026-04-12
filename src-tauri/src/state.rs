use crate::mcp::EmbeddedMcpServer;
use crate::model::{
    apply_connection_snapshot, ConnectRequest, McpServerStatus, SendRequest, SessionSnapshot,
    SourceKind, UiBusEvent, UiConnectionState, UiTransportKind,
};
use eadai::bus::BusSubscription;
use eadai::cli::{ParserKind, RunConfig};
use eadai::runtime_host::{FakeRuntimeConfig, RuntimeSessionConfig, SessionRuntimeHost};
use eadai::serial;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const SERIAL_EVENT_NAME: &str = "serial-bus-event";

pub struct DesktopState {
    runtime: SessionRuntimeHost,
    mcp_server: EmbeddedMcpServer,
    inner: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    running: Option<RunningSession>,
    last_snapshot: SessionSnapshot,
}

struct RunningSession {
    snapshot: Arc<Mutex<SessionSnapshot>>,
    forwarder: JoinHandle<()>,
}

impl Default for DesktopState {
    fn default() -> Self {
        let runtime = SessionRuntimeHost::default();
        let mcp_server = EmbeddedMcpServer::new(runtime.adapter());
        Self {
            runtime,
            mcp_server,
            inner: Mutex::new(SessionState::default()),
        }
    }
}

impl DesktopState {
    pub fn list_ports(&self) -> Result<Vec<String>, String> {
        self.runtime.list_ports().map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let state = lock_state(&self.inner);

        match &state.running {
            Some(session) => lock_snapshot(&session.snapshot).clone(),
            None => state.last_snapshot.clone(),
        }
    }

    pub fn mcp_status(&self) -> McpServerStatus {
        self.mcp_server.status()
    }

    pub fn connect(
        &self,
        app_handle: &AppHandle,
        request: ConnectRequest,
    ) -> Result<SessionSnapshot, String> {
        let snapshot = Arc::new(Mutex::new(initial_snapshot(&request)));
        let subscription = self
            .runtime
            .connect(runtime_config(&request))
            .map_err(|error| error.to_string())?;
        let forwarder = spawn_forwarder(app_handle.clone(), subscription, Arc::clone(&snapshot));
        let current_snapshot = lock_snapshot(&snapshot).clone();

        let mut state = lock_state(&self.inner);
        state.last_snapshot = current_snapshot.clone();
        state.running = Some(RunningSession {
            snapshot,
            forwarder,
        });

        Ok(current_snapshot)
    }

    pub fn disconnect(&self) -> Result<SessionSnapshot, String> {
        let running = {
            let mut state = lock_state(&self.inner);
            state.running.take()
        };

        let running =
            running.ok_or_else(|| runtime_state_error("runtime session is not running"))?;

        if let Err(error) = self.runtime.disconnect() {
            let mut state = lock_state(&self.inner);
            state.running = Some(running);
            return Err(error.to_string());
        }

        let _ = running.forwarder.join();

        let mut snapshot = lock_snapshot(&running.snapshot).clone();
        snapshot.is_running = false;
        snapshot.connection_state = Some(UiConnectionState::Stopped);

        let mut state = lock_state(&self.inner);
        state.last_snapshot = snapshot.clone();
        Ok(snapshot)
    }

    pub fn send(&self, request: SendRequest) -> Result<(), String> {
        self.runtime
            .send_payload(serial::payload_bytes_for_text(
                &request.payload,
                request.append_newline,
            ))
            .map_err(|error| error.to_string())
    }
}

fn runtime_state_error(message: &str) -> String {
    std::io::Error::new(std::io::ErrorKind::NotConnected, message).to_string()
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
            crate::fake_session::fake_port_label(
                request
                    .fake_profile
                    .as_deref()
                    .unwrap_or(crate::fake_session::default_profile()),
            ),
            request.baud_rate,
        ),
    }
}

fn runtime_config(request: &ConnectRequest) -> RuntimeSessionConfig {
    match request.source_kind {
        SourceKind::Serial => RuntimeSessionConfig::Serial(RunConfig {
            port: request.port.clone(),
            baud_rate: request.baud_rate,
            retry_delay: Duration::from_millis(request.retry_ms),
            read_timeout: Duration::from_millis(request.read_timeout_ms),
            parser: ParserKind::Auto,
            max_frame_bytes: eadai::cli::DEFAULT_MAX_FRAME_BYTES,
        }),
        SourceKind::Fake => RuntimeSessionConfig::Fake(FakeRuntimeConfig {
            profile: request
                .fake_profile
                .clone()
                .unwrap_or_else(|| crate::fake_session::default_profile().to_string()),
            baud_rate: request.baud_rate,
        }),
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
