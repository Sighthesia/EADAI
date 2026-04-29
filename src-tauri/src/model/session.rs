use eadai::message::{ConnectionEvent, ConnectionState, MessageSource, TransportKind};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolUsageSnapshot {
    pub name: String,
    pub last_called_at_ms: Option<u64>,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSource {
    pub transport: UiTransportKind,
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConnectionPayload {
    pub state: UiConnectionState,
    pub reason: Option<String>,
    pub attempt: u32,
    pub retry_delay_ms: Option<u64>,
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

impl From<MessageSource> for UiSource {
    fn from(value: MessageSource) -> Self {
        Self {
            transport: value.transport.into(),
            port: value.port,
            baud_rate: value.baud_rate,
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
