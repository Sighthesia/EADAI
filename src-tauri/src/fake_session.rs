use eadai::bus::MessageBus;
use eadai::cli::ParserKind;
use eadai::message::{BusMessage, ConnectionState, LinePayload, MessageSource};
use eadai::parser;
use eadai::serial::{payload_bytes_for_text, FrameStatus, FramedLine};
use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const DEFAULT_FAKE_PROFILE: &str = "telemetry-lab";
const DEFAULT_EMIT_INTERVAL_MS: u64 = 100;
const COMMAND_ACK_DELAY_MS: u64 = 45;

enum FakeCommand {
    Send { payload: Vec<u8> },
}

#[derive(Clone)]
pub struct FakeSessionHandle {
    stop_requested: Arc<AtomicBool>,
    command_tx: Sender<FakeCommand>,
}

impl FakeSessionHandle {
    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    pub fn send_payload(&self, payload: Vec<u8>) -> Result<(), String> {
        self.command_tx
            .send(FakeCommand::Send { payload })
            .map_err(|_| "fake session command channel is closed".to_string())
    }
}

pub struct FakeSessionConfig {
    pub port: String,
    pub baud_rate: u32,
    pub profile: String,
}

pub fn default_profile() -> &'static str {
    DEFAULT_FAKE_PROFILE
}

pub fn fake_port_label(profile: &str) -> String {
    format!("fake://{profile}")
}

pub fn spawn(config: FakeSessionConfig, bus: MessageBus) -> (FakeSessionHandle, JoinHandle<()>) {
    let stop_requested = Arc::new(AtomicBool::new(false));
    let (command_tx, command_rx) = mpsc::channel();
    let handle = FakeSessionHandle {
        stop_requested: Arc::clone(&stop_requested),
        command_tx,
    };

    let worker = thread::spawn(move || {
        let source = MessageSource::fake(config.port, config.baud_rate);
        bus.publish(BusMessage::connection(
            &source,
            ConnectionState::Idle,
            None,
            0,
            None,
        ));
        bus.publish(BusMessage::connection(
            &source,
            ConnectionState::Connecting,
            None,
            1,
            None,
        ));
        bus.publish(BusMessage::connection(
            &source,
            ConnectionState::Connected,
            None,
            1,
            None,
        ));

        let started_at = Instant::now();
        let mut sample_index = 0_u64;
        let mut next_emit_at = Instant::now();

        loop {
            if stop_requested.load(Ordering::SeqCst) {
                bus.publish(BusMessage::connection(
                    &source,
                    ConnectionState::Stopped,
                    Some("stop requested".to_string()),
                    1,
                    None,
                ));
                return;
            }

            loop {
                match command_rx.try_recv() {
                    Ok(FakeCommand::Send { payload }) => {
                        let outbound = line_payload_from_bytes(&payload);
                        bus.publish(BusMessage::tx_line(&source, outbound.clone()));

                        thread::sleep(Duration::from_millis(COMMAND_ACK_DELAY_MS));
                        publish_rx(
                            &bus,
                            &source,
                            &format!("command_ack=1 command_len={}", outbound.raw.len()),
                        );
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            if Instant::now() >= next_emit_at {
                for line in profile_lines(&config.profile, sample_index, started_at.elapsed()) {
                    publish_rx(&bus, &source, &line);
                }

                sample_index += 1;
                next_emit_at += Duration::from_millis(DEFAULT_EMIT_INTERVAL_MS);
            }

            thread::sleep(Duration::from_millis(16));
        }
    });

    (handle, worker)
}

fn publish_rx(bus: &MessageBus, source: &MessageSource, text: &str) {
    let payload = LinePayload {
        text: text.to_string(),
        raw: payload_bytes_for_text(text, false),
    };
    let parser = parser::parse_framed_line(
        ParserKind::Auto,
        &FramedLine {
            payload: payload.clone(),
            status: FrameStatus::Complete,
        },
    );
    bus.publish(BusMessage::rx_line(source, payload).with_parser(parser));
}

fn line_payload_from_bytes(payload: &[u8]) -> LinePayload {
    let mut raw = payload.to_vec();

    while matches!(raw.last(), Some(b'\n' | b'\r')) {
        raw.pop();
    }

    let text = String::from_utf8_lossy(&raw).into_owned();
    LinePayload { text, raw }
}

fn profile_lines(profile: &str, sample_index: u64, elapsed: Duration) -> Vec<String> {
    match profile {
        "noisy-monitor" => noisy_monitor_lines(sample_index, elapsed),
        _ => telemetry_lab_lines(sample_index, elapsed),
    }
}

fn telemetry_lab_lines(sample_index: u64, elapsed: Duration) -> Vec<String> {
    let phase = sample_index as f64 * 0.18;
    let temp = 24.5 + phase.sin() * 2.4;
    let voltage = 12.1 + (phase * 0.7).cos() * 0.28;
    let rpm = 1480.0 + (phase * 1.1).sin() * 165.0;
    let current_ma = 820.0 + (phase * 0.9).sin() * 95.0;
    let flow_lpm = 13.5 + (phase * 0.4).cos() * 1.6;
    let vibration = 0.42 + (phase * 2.2).sin().abs() * 0.18;
    let humidity = 46.0 + (phase * 0.23).sin() * 5.5;
    let torque_nm = 18.0 + (phase * 0.8).sin() * 2.7;
    let power_w = voltage * (current_ma / 1000.0);
    let timestamp = elapsed.as_millis();

    vec![
        format!("timestamp={timestamp} temp={temp:.2}"),
        format!("timestamp={timestamp} voltage={voltage:.2}"),
        format!("timestamp={timestamp} motor_rpm={rpm:.0}"),
        format!("timestamp={timestamp} current_ma={current_ma:.0}"),
        format!("timestamp={timestamp} flow_lpm={flow_lpm:.2}"),
        format!("timestamp={timestamp} vibration_g={vibration:.3}"),
        format!("timestamp={timestamp} humidity_pct={humidity:.1}"),
        format!("timestamp={timestamp} torque_nm={torque_nm:.2}"),
        format!("timestamp={timestamp} power_w={power_w:.2}"),
    ]
}

fn noisy_monitor_lines(sample_index: u64, elapsed: Duration) -> Vec<String> {
    let phase = sample_index as f64 * (PI / 14.0);
    let temp = 26.0 + phase.sin() * 1.8;
    let pressure = 101.2 + (phase * 0.5).cos() * 0.35;
    let load = 48.0 + (phase * 1.3).sin() * 12.0;
    let bus_voltage = 23.8 + (phase * 0.25).sin() * 1.2;
    let current_ma = 1440.0 + (phase * 1.2).cos() * 280.0;
    let vibration = 0.18 + (phase * 3.1).sin().abs() * 0.4;
    let humidity = 51.0 + (phase * 0.35).cos() * 7.5;
    let flow_lpm = 9.4 + (phase * 0.9).sin() * 1.9;
    let timestamp = elapsed.as_millis();

    let mut lines = vec![
        format!("timestamp={timestamp} temp={temp:.2}"),
        format!("timestamp={timestamp} pressure={pressure:.2}"),
        format!("timestamp={timestamp} load_pct={load:.1}"),
        format!("timestamp={timestamp} bus_voltage={bus_voltage:.2}"),
        format!("timestamp={timestamp} current_ma={current_ma:.0}"),
        format!("timestamp={timestamp} vibration_g={vibration:.3}"),
        format!("timestamp={timestamp} humidity_pct={humidity:.1}"),
        format!("timestamp={timestamp} flow_lpm={flow_lpm:.2}"),
    ];

    if sample_index.is_multiple_of(5) {
        lines.push("boot heartbeat".to_string());
    }

    if sample_index.is_multiple_of(7) {
        lines.push("temp= WARN".to_string());
    }

    lines
}
