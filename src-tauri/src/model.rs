use eadai::analysis::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use eadai::message::{
    BusMessage, ConnectionEvent, ConnectionState, LineDirection, MessageKind, MessageSource,
    ParserMeta, TransportKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    #[default]
    Serial,
    Fake,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectRequest {
    pub port: String,
    pub baud_rate: u32,
    pub retry_ms: u64,
    pub read_timeout_ms: u64,
    #[serde(default)]
    pub source_kind: SourceKind,
    #[serde(default)]
    pub fake_profile: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub payload: String,
    pub append_newline: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub is_running: bool,
    pub transport: Option<UiTransportKind>,
    pub port: Option<String>,
    pub baud_rate: Option<u32>,
    pub connection_state: Option<UiConnectionState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerStatus {
    pub is_running: bool,
    pub transport: String,
    pub endpoint_url: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureRequest {
    pub device_ref: String,
    pub sample_count: u32,
    pub samplerate_hz: Option<u64>,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerDevice {
    pub reference: String,
    pub name: String,
    pub driver: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    pub note: Option<String>,
    pub raw_line: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureState {
    pub pid: u32,
    pub started_at_ms: u64,
    pub command: String,
    pub output_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerWaveformChannel {
    pub label: String,
    pub samples: Vec<Option<bool>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerCaptureResult {
    pub output_path: String,
    pub sample_rate_hz: Option<u64>,
    pub sample_count: usize,
    pub channels: Vec<LogicAnalyzerWaveformChannel>,
    pub captured_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogicAnalyzerSessionState {
    Unavailable,
    Idle,
    Scanning,
    Ready,
    Capturing,
    Stopping,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicAnalyzerStatus {
    pub available: bool,
    pub executable: Option<String>,
    pub session_state: LogicAnalyzerSessionState,
    pub devices: Vec<LogicAnalyzerDevice>,
    pub selected_device_ref: Option<String>,
    pub active_capture: Option<LogicAnalyzerCaptureState>,
    pub last_capture: Option<LogicAnalyzerCaptureResult>,
    pub last_scan_at_ms: Option<u64>,
    pub scan_output: Option<String>,
    pub last_error: Option<String>,
    pub capture_plan: Option<String>,
    pub linux_first_note: String,
}

impl Default for LogicAnalyzerStatus {
    fn default() -> Self {
        Self {
            available: false,
            executable: None,
            session_state: LogicAnalyzerSessionState::Idle,
            devices: Vec::new(),
            selected_device_ref: None,
            active_capture: None,
            last_capture: None,
            last_scan_at_ms: None,
            scan_output: None,
            last_error: None,
            capture_plan: None,
            linux_first_note:
                "Linux-first sigrok path; install `sigrok-cli` or set `EADAI_SIGROK_CLI`."
                    .to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiTransportKind {
    Serial,
    Fake,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiConnectionState {
    Idle,
    Connecting,
    Connected,
    WaitingRetry,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiLineDirection {
    Rx,
    Tx,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UiTriggerSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSource {
    pub transport: UiTransportKind,
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiParserMeta {
    pub parser_name: Option<String>,
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConnectionPayload {
    pub state: UiConnectionState,
    pub reason: Option<String>,
    pub attempt: u32,
    pub retry_delay_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLinePayload {
    pub direction: UiLineDirection,
    pub text: String,
    pub raw_length: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAnalysisPayload {
    pub channel_id: String,
    pub window_ms: u64,
    pub sample_count: usize,
    pub time_span_ms: Option<f64>,
    pub frequency_hz: Option<f64>,
    pub period_ms: Option<f64>,
    pub period_stability: Option<f64>,
    pub duty_cycle: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub mean_value: Option<f64>,
    pub rms_value: Option<f64>,
    pub variance: Option<f64>,
    pub edge_count: usize,
    pub rising_edge_count: usize,
    pub falling_edge_count: usize,
    pub trend: Option<f64>,
    pub change_rate: Option<f64>,
    pub trigger_hits: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTriggerPayload {
    pub channel_id: String,
    pub rule_id: String,
    pub severity: UiTriggerSeverity,
    pub fired_at_ms: u64,
    pub reason: String,
    pub snapshot: Option<UiAnalysisPayload>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UiBusEvent {
    Connection {
        timestamp_ms: u64,
        source: UiSource,
        connection: UiConnectionPayload,
    },
    Line {
        timestamp_ms: u64,
        source: UiSource,
        line: UiLinePayload,
        parser: UiParserMeta,
    },
    Analysis {
        timestamp_ms: u64,
        source: UiSource,
        analysis: UiAnalysisPayload,
    },
    Trigger {
        timestamp_ms: u64,
        source: UiSource,
        trigger: UiTriggerPayload,
    },
}

impl SessionSnapshot {
    pub fn connecting(transport: UiTransportKind, port: String, baud_rate: u32) -> Self {
        Self {
            is_running: true,
            transport: Some(transport),
            port: Some(port),
            baud_rate: Some(baud_rate),
            connection_state: Some(UiConnectionState::Idle),
        }
    }
}

impl McpServerStatus {
    pub fn starting() -> Self {
        Self {
            is_running: false,
            transport: "streamableHttp".to_string(),
            endpoint_url: None,
            last_error: None,
        }
    }

    pub fn running(endpoint_url: String, last_error: Option<String>) -> Self {
        Self {
            is_running: true,
            transport: "streamableHttp".to_string(),
            endpoint_url: Some(endpoint_url),
            last_error,
        }
    }

    pub fn failed(error: String) -> Self {
        Self {
            is_running: false,
            transport: "streamableHttp".to_string(),
            endpoint_url: None,
            last_error: Some(error),
        }
    }
}

impl From<TransportKind> for UiTransportKind {
    fn from(value: TransportKind) -> Self {
        match value {
            TransportKind::Serial => Self::Serial,
            TransportKind::Fake => Self::Fake,
        }
    }
}

impl From<ConnectionState> for UiConnectionState {
    fn from(value: ConnectionState) -> Self {
        match value {
            ConnectionState::Idle => Self::Idle,
            ConnectionState::Connecting => Self::Connecting,
            ConnectionState::Connected => Self::Connected,
            ConnectionState::WaitingRetry => Self::WaitingRetry,
            ConnectionState::Stopped => Self::Stopped,
        }
    }
}

impl From<LineDirection> for UiLineDirection {
    fn from(value: LineDirection) -> Self {
        match value {
            LineDirection::Rx => Self::Rx,
            LineDirection::Tx => Self::Tx,
        }
    }
}

impl From<TriggerSeverity> for UiTriggerSeverity {
    fn from(value: TriggerSeverity) -> Self {
        match value {
            TriggerSeverity::Info => Self::Info,
            TriggerSeverity::Warning => Self::Warning,
            TriggerSeverity::Critical => Self::Critical,
        }
    }
}

impl From<MessageSource> for UiSource {
    fn from(value: MessageSource) -> Self {
        Self {
            transport: value.transport.into(),
            port: value.port,
            baud_rate: value.baud_rate,
        }
    }
}

impl From<ParserMeta> for UiParserMeta {
    fn from(value: ParserMeta) -> Self {
        Self {
            parser_name: value.parser_name,
            fields: value
                .fields
                .into_iter()
                .map(|(key, value)| (normalize_parser_key(&key), value))
                .collect(),
        }
    }
}

impl From<ConnectionEvent> for UiConnectionPayload {
    fn from(value: ConnectionEvent) -> Self {
        Self {
            state: value.state.into(),
            reason: value.reason,
            attempt: value.attempt,
            retry_delay_ms: value.retry_delay_ms,
        }
    }
}

impl From<AnalysisFrame> for UiAnalysisPayload {
    fn from(value: AnalysisFrame) -> Self {
        Self {
            channel_id: value.channel_id,
            window_ms: value.window_ms,
            sample_count: value.sample_count,
            time_span_ms: value.time_span_ms,
            frequency_hz: value.frequency_hz,
            period_ms: value.period_ms,
            period_stability: value.period_stability,
            duty_cycle: value.duty_cycle,
            min_value: value.min_value,
            max_value: value.max_value,
            mean_value: value.mean_value,
            rms_value: value.rms_value,
            variance: value.variance,
            edge_count: value.edge_count,
            rising_edge_count: value.rising_edge_count,
            falling_edge_count: value.falling_edge_count,
            trend: value.trend,
            change_rate: value.change_rate,
            trigger_hits: value.trigger_hits,
        }
    }
}

impl From<TriggerEvent> for UiTriggerPayload {
    fn from(value: TriggerEvent) -> Self {
        Self {
            channel_id: value.channel_id,
            rule_id: value.rule_id,
            severity: value.severity.into(),
            fired_at_ms: value.fired_at_ms,
            reason: value.reason,
            snapshot: value.snapshot.map(Into::into),
        }
    }
}

impl From<BusMessage> for UiBusEvent {
    fn from(value: BusMessage) -> Self {
        let BusMessage {
            timestamp,
            source,
            kind,
            parser,
        } = value;
        let timestamp_ms = timestamp_ms(timestamp);
        let source = UiSource::from(source);

        match kind {
            MessageKind::Connection(connection) => Self::Connection {
                timestamp_ms,
                source,
                connection: connection.into(),
            },
            MessageKind::Line(line) => Self::Line {
                timestamp_ms,
                source,
                line: UiLinePayload {
                    direction: line.direction.into(),
                    raw_length: line.payload.raw.len(),
                    text: line.payload.text,
                },
                parser: parser.into(),
            },
            MessageKind::Analysis(frame) => Self::Analysis {
                timestamp_ms,
                source,
                analysis: frame.into(),
            },
            MessageKind::Trigger(trigger) => Self::Trigger {
                timestamp_ms,
                source,
                trigger: trigger.into(),
            },
        }
    }
}

pub fn apply_connection_snapshot(
    snapshot: &mut SessionSnapshot,
    event: &UiConnectionPayload,
    source: &UiSource,
) {
    snapshot.is_running = event.state != UiConnectionState::Stopped;
    snapshot.transport = Some(source.transport.clone());
    snapshot.port = Some(source.port.clone());
    snapshot.baud_rate = Some(source.baud_rate);
    snapshot.connection_state = Some(event.state.clone());
}

fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn normalize_parser_key(key: &str) -> String {
    match key {
        "channel_id" => "channelId".to_string(),
        "numeric_value" => "numericValue".to_string(),
        "field_count" => "fieldCount".to_string(),
        other => other.to_string(),
    }
}
