use std::collections::BTreeMap;
use std::time::SystemTime;

/// Transport kinds supported by the message bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Serial,
}

/// Source metadata attached to every bus message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageSource {
    pub transport: TransportKind,
    pub port: String,
    pub baud_rate: u32,
}

impl MessageSource {
    /// Creates serial source metadata.
    ///
    /// - `port`: serial port name.
    /// - `baud_rate`: configured baud rate.
    pub fn serial(port: impl Into<String>, baud_rate: u32) -> Self {
        Self {
            transport: TransportKind::Serial,
            port: port.into(),
            baud_rate,
        }
    }
}

/// Connection lifecycle states emitted through the bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    WaitingRetry,
    Stopped,
}

/// Connection event payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionEvent {
    pub state: ConnectionState,
    pub reason: Option<String>,
    pub attempt: u32,
    pub retry_delay_ms: Option<u64>,
}

/// Line-oriented serial payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinePayload {
    pub text: String,
    pub raw: Vec<u8>,
}

/// Direction of one serial line event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineDirection {
    Rx,
    Tx,
}

/// Serial line event with explicit direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineEvent {
    pub direction: LineDirection,
    pub payload: LinePayload,
}

/// Parser outcome attached to each line message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParserStatus {
    #[default]
    Unparsed,
    Parsed,
    Malformed,
}

/// Parser metadata reserved for downstream protocol consumers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParserMeta {
    pub parser_name: Option<String>,
    pub status: ParserStatus,
    pub fields: BTreeMap<String, String>,
}

impl ParserMeta {
    /// Creates parsed metadata.
    ///
    /// - `parser_name`: parser identifier.
    /// - `fields`: normalized parser output.
    pub fn parsed(parser_name: impl Into<String>, fields: BTreeMap<String, String>) -> Self {
        Self {
            parser_name: Some(parser_name.into()),
            status: ParserStatus::Parsed,
            fields,
        }
    }

    /// Creates malformed metadata with a normalized reason.
    ///
    /// - `parser_name`: parser identifier when available.
    /// - `reason`: normalized failure reason.
    pub fn malformed(parser_name: Option<&str>, reason: impl Into<String>) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert("error".to_string(), reason.into());

        Self {
            parser_name: parser_name.map(str::to_string),
            status: ParserStatus::Malformed,
            fields,
        }
    }

    /// Creates unparsed metadata.
    pub fn unparsed() -> Self {
        Self::default()
    }
}

/// Top-level message payload variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Connection(ConnectionEvent),
    Line(LineEvent),
}

/// Envelope broadcast to downstream consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusMessage {
    pub timestamp: SystemTime,
    pub source: MessageSource,
    pub kind: MessageKind,
    pub parser: ParserMeta,
}

impl BusMessage {
    /// Creates a connection-state message.
    ///
    /// - `source`: transport metadata.
    /// - `state`: new connection state.
    /// - `reason`: optional failure or shutdown reason.
    /// - `attempt`: current connect attempt number.
    /// - `retry_delay_ms`: optional retry backoff in milliseconds.
    pub fn connection(
        source: &MessageSource,
        state: ConnectionState,
        reason: Option<String>,
        attempt: u32,
        retry_delay_ms: Option<u64>,
    ) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Connection(ConnectionEvent {
                state,
                reason,
                attempt,
                retry_delay_ms,
            }),
            parser: ParserMeta::default(),
        }
    }

    /// Creates a line payload message.
    ///
    /// - `source`: transport metadata.
    /// - `direction`: whether this line was received or transmitted.
    /// - `payload`: line payload without trailing newline bytes.
    pub fn line(source: &MessageSource, direction: LineDirection, payload: LinePayload) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Line(LineEvent { direction, payload }),
            parser: ParserMeta::default(),
        }
    }

    /// Creates a received line payload message.
    ///
    /// - `source`: transport metadata.
    /// - `payload`: line payload without trailing newline bytes.
    pub fn rx_line(source: &MessageSource, payload: LinePayload) -> Self {
        Self::line(source, LineDirection::Rx, payload)
    }

    /// Creates a transmitted line payload message.
    ///
    /// - `source`: transport metadata.
    /// - `payload`: line payload without trailing newline bytes.
    pub fn tx_line(source: &MessageSource, payload: LinePayload) -> Self {
        Self::line(source, LineDirection::Tx, payload)
    }

    /// Replaces parser metadata on an existing message.
    ///
    /// - `parser`: parsed metadata generated by a protocol parser.
    pub fn with_parser(mut self, parser: ParserMeta) -> Self {
        self.parser = parser;
        self
    }
}
