use crate::logic_analyzer::LogicAnalyzerService;
use crate::mcp::EmbeddedMcpServer;
use crate::model::{
    apply_connection_snapshot, Bmi088CommandRequest, ConnectRequest, LogicAnalyzerCaptureRequest,
    LogicAnalyzerStatus, McpServerStatus, McpToolUsageSnapshot, SendRequest, SessionSnapshot,
    SourceKind, UiBusEvent, UiConnectionState, UiTransportKind,
};
use eadai::bus::BusSubscription;
use eadai::cli::{ParserKind, RunConfig};
use eadai::runtime_host::{FakeRuntimeConfig, RuntimeSessionConfig, SessionRuntimeHost};
use eadai::serial::{self, SerialDeviceInfo};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;

#[cfg(target_os = "linux")]
use udev::{EventType, MonitorBuilder};

pub const SERIAL_EVENT_NAME: &str = "serial-bus-event";
pub const SERIAL_DEVICE_EVENT_NAME: &str = "serial-devices-changed";

pub struct DesktopState {
    runtime: SessionRuntimeHost,
    mcp_server: EmbeddedMcpServer,
    logic_analyzer: LogicAnalyzerService,
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
        let logic_analyzer = LogicAnalyzerService::default();
        Self {
            runtime,
            mcp_server,
            logic_analyzer,
            inner: Mutex::new(SessionState::default()),
        }
    }
}

impl DesktopState {
    pub fn start_serial_device_watcher(&self, app_handle: AppHandle) {
        start_serial_device_watcher_thread(app_handle);
    }

    pub fn list_ports(&self) -> Result<Vec<SerialDeviceInfo>, String> {
        self.runtime
            .list_visible_devices()
            .map_err(|error| error.to_string())
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

    pub fn mcp_tool_usage_snapshot(&self) -> Vec<McpToolUsageSnapshot> {
        self.mcp_server.tool_usage_snapshot()
    }

    pub fn logic_analyzer_status(&self) -> LogicAnalyzerStatus {
        self.logic_analyzer.status()
    }

    pub fn refresh_logic_analyzer_devices(&self) -> Result<LogicAnalyzerStatus, String> {
        self.logic_analyzer.refresh_devices()
    }

    pub fn start_logic_analyzer_capture(
        &self,
        request: LogicAnalyzerCaptureRequest,
    ) -> Result<LogicAnalyzerStatus, String> {
        self.logic_analyzer.start_capture(request)
    }

    pub fn stop_logic_analyzer_capture(&self) -> Result<LogicAnalyzerStatus, String> {
        self.logic_analyzer.stop_capture()
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

    pub fn send_bmi088_command(&self, request: Bmi088CommandRequest) -> Result<(), String> {
        self.runtime
            .send_bmi088_command(
                request.command.clone().into(),
                request.command.payload_bytes(request.payload.as_deref()),
            )
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
            parser: ParserKind::Bmi088,
            max_frame_bytes: eadai::cli::DEFAULT_MAX_FRAME_BYTES,
            transport: eadai::cli::TransportSelection::Serial,
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

            if let UiBusEvent::TelemetrySchema { source, schema, .. } = &event {
                let _ = (source, schema);
            }

            let _ = app_handle.emit(SERIAL_EVENT_NAME, &event);
        }
    })
}

#[cfg(target_os = "linux")]
fn start_serial_device_watcher_thread(app_handle: AppHandle) {
    thread::spawn(move || {
        let socket = match MonitorBuilder::new()
            .and_then(|builder| builder.match_subsystem("tty"))
            .and_then(|builder| builder.listen())
        {
            Ok(socket) => socket,
            Err(error) => {
                eprintln!("failed to start udev serial monitor: {error}");
                return;
            }
        };

        let mut descriptors = [libc::pollfd {
            fd: socket.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];

        loop {
            let result = unsafe {
                libc::poll(
                    descriptors.as_mut_ptr(),
                    descriptors.len() as libc::nfds_t,
                    -1,
                )
            };

            if result < 0 {
                let error = std::io::Error::last_os_error();
                eprintln!("udev serial monitor poll failed: {error}");
                break;
            }

            if result == 0 {
                continue;
            }

            for event in socket.iter() {
                if !matches!(
                    event.event_type(),
                    EventType::Add | EventType::Change | EventType::Remove
                ) {
                    continue;
                }

                let _ = app_handle.emit(SERIAL_DEVICE_EVENT_NAME, ());
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn start_serial_device_watcher_thread(_app_handle: AppHandle) {}

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
