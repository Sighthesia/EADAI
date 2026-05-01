use crate::analysis::{AnalysisFrame, TriggerEvent};
use crate::bmi088::{Bmi088IdentityFrame, Bmi088SampleFrame, Bmi088SchemaFrame};
use crate::protocols::self_describing::frame::{
    AckResult, CommandCatalogPage, Identity as SelfDescribingIdentity, SetVariable,
    TelemetrySample as SelfDescribingSample, VariableCatalogPage,
};
use crate::protocols::{CapabilityEvent, CrtpPacket, MavlinkPacket};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum TransportKind {
    Serial,
    Fake,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MessageSource {
    pub transport: TransportKind,
    pub port: String,
    pub baud_rate: u32,
}

impl MessageSource {
    pub fn serial(port: impl Into<String>, baud_rate: u32) -> Self {
        Self {
            transport: TransportKind::Serial,
            port: port.into(),
            baud_rate,
        }
    }

    pub fn fake(port: impl Into<String>, baud_rate: u32) -> Self {
        Self {
            transport: TransportKind::Fake,
            port: port.into(),
            baud_rate,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ConnectionState {
    Idle,
    Connecting,
    Connected,
    WaitingRetry,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConnectionEvent {
    pub state: ConnectionState,
    pub reason: Option<String>,
    pub attempt: u32,
    pub retry_delay_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinePayload {
    pub text: String,
    pub raw: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum LineDirection {
    Rx,
    Tx,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LineEvent {
    pub direction: LineDirection,
    pub payload: LinePayload,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum ParserStatus {
    #[default]
    Unparsed,
    Parsed,
    Malformed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ParserMeta {
    pub parser_name: Option<String>,
    pub status: ParserStatus,
    pub fields: BTreeMap<String, String>,
}

impl ParserMeta {
    pub fn parsed(parser_name: impl Into<String>, fields: BTreeMap<String, String>) -> Self {
        Self {
            parser_name: Some(parser_name.into()),
            status: ParserStatus::Parsed,
            fields,
        }
    }

    pub fn malformed(parser_name: Option<&str>, reason: impl Into<String>) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert("error".to_string(), reason.into());

        Self {
            parser_name: parser_name.map(str::to_string),
            status: ParserStatus::Malformed,
            fields,
        }
    }

    pub fn unparsed() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageKind {
    Connection(ConnectionEvent),
    Line(LineEvent),
    ShellOutput(LineEvent),
    TelemetryIdentity(Bmi088IdentityFrame),
    TelemetrySchema(Bmi088SchemaFrame),
    TelemetrySample(Bmi088SampleFrame),
    MavlinkPacket(MavlinkPacket),
    CrtpPacket(CrtpPacket),
    Capability(CapabilityEvent),
    Analysis(AnalysisFrame),
    Trigger(TriggerEvent),
    /// Self-describing protocol: device identity received.
    SelfDescribingIdentity(SelfDescribingIdentity),
    /// Self-describing protocol: variable catalog page received.
    SelfDescribingVariableCatalog(VariableCatalogPage),
    /// Self-describing protocol: command catalog page received.
    SelfDescribingCommandCatalog(CommandCatalogPage),
    /// Self-describing protocol: telemetry sample received.
    SelfDescribingSample(SelfDescribingSample),
    /// Self-describing protocol: set variable request from host.
    SelfDescribingSetVariable(SetVariable),
    /// Self-describing protocol: acknowledgment/result from device.
    SelfDescribingAckResult(AckResult),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusMessage {
    pub timestamp: SystemTime,
    pub source: MessageSource,
    pub kind: MessageKind,
    pub parser: ParserMeta,
}

impl BusMessage {
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

    pub fn line(source: &MessageSource, direction: LineDirection, payload: LinePayload) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Line(LineEvent { direction, payload }),
            parser: ParserMeta::default(),
        }
    }

    pub fn rx_line(source: &MessageSource, payload: LinePayload) -> Self {
        Self::line(source, LineDirection::Rx, payload)
    }

    pub fn tx_line(source: &MessageSource, payload: LinePayload) -> Self {
        Self::line(source, LineDirection::Tx, payload)
    }

    pub fn shell_output(source: &MessageSource, payload: LinePayload) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::ShellOutput(LineEvent {
                direction: LineDirection::Rx,
                payload,
            }),
            parser: ParserMeta::default(),
        }
    }

    pub fn with_parser(mut self, parser: ParserMeta) -> Self {
        self.parser = parser;
        self
    }

    pub fn analysis(source: &MessageSource, frame: AnalysisFrame) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Analysis(frame),
            parser: ParserMeta::default(),
        }
    }

    pub fn trigger(source: &MessageSource, trigger: TriggerEvent) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Trigger(trigger),
            parser: ParserMeta::default(),
        }
    }

    pub fn telemetry_schema(source: &MessageSource, schema: Bmi088SchemaFrame) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::TelemetrySchema(schema),
            parser: ParserMeta::default(),
        }
    }

    pub fn telemetry_identity(source: &MessageSource, identity: Bmi088IdentityFrame) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::TelemetryIdentity(identity),
            parser: ParserMeta::default(),
        }
    }

    pub fn telemetry_sample(source: &MessageSource, sample: Bmi088SampleFrame) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::TelemetrySample(sample),
            parser: ParserMeta::default(),
        }
    }

    pub fn mavlink_packet(source: &MessageSource, packet: MavlinkPacket) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::MavlinkPacket(packet),
            parser: ParserMeta::default(),
        }
    }

    pub fn crtp_packet(source: &MessageSource, packet: CrtpPacket) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::CrtpPacket(packet),
            parser: ParserMeta::default(),
        }
    }

    pub fn capability(source: &MessageSource, event: CapabilityEvent) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::Capability(event),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_identity(
        source: &MessageSource,
        identity: SelfDescribingIdentity,
    ) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingIdentity(identity),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_variable_catalog(
        source: &MessageSource,
        catalog: VariableCatalogPage,
    ) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingVariableCatalog(catalog),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_command_catalog(
        source: &MessageSource,
        catalog: CommandCatalogPage,
    ) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingCommandCatalog(catalog),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_sample(source: &MessageSource, sample: SelfDescribingSample) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingSample(sample),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_set_variable(source: &MessageSource, set_var: SetVariable) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingSetVariable(set_var),
            parser: ParserMeta::default(),
        }
    }

    pub fn self_describing_ack_result(source: &MessageSource, result: AckResult) -> Self {
        Self {
            timestamp: SystemTime::now(),
            source: source.clone(),
            kind: MessageKind::SelfDescribingAckResult(result),
            parser: ParserMeta::default(),
        }
    }
}
