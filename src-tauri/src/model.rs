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

impl From<BusMessage> for UiBusEvent {
    fn from(value: BusMessage) -> Self {
        let timestamp_ms = timestamp_ms(value.timestamp);
        let source = UiSource::from(value.source);

        match value.kind {
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
                parser: value.parser.into(),
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
        other => other.to_string(),
    }
}
