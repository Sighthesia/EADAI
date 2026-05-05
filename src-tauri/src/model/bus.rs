use eadai::bmi088::{
    Bmi088IdentityFrame, Bmi088SampleFrame, Bmi088SchemaFrame, encode_identity_frame,
    encode_sample_frame, encode_schema_frame,
};
use eadai::message::{BusMessage, MessageKind};
use eadai::protocols::self_describing::frame::{
    AckResult, CommandCatalogPage, Identity as SelfDescribingIdentity, SetVariable,
    TelemetrySample as SelfDescribingSample, VariableCatalogPage,
};
use eadai::protocols::self_describing::SelfDescribingStreamingDriftVerdict;
use eadai::protocols::{CapabilityEvent, CrtpPacket, MavlinkPacket};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use super::line::{UiLinePayload, UiParserMeta};
use super::protocol::{UiAnalysisPayload, UiTriggerPayload};
use super::session::UiSource;

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum UiBusEvent {
    Connection {
        timestamp_ms: u64,
        source: UiSource,
        connection: super::session::UiConnectionPayload,
    },
    Line {
        timestamp_ms: u64,
        source: UiSource,
        line: UiLinePayload,
        parser: UiParserMeta,
    },
    ShellOutput {
        timestamp_ms: u64,
        source: UiSource,
        line: UiLinePayload,
        parser: UiParserMeta,
    },
    TelemetrySchema {
        timestamp_ms: u64,
        source: UiSource,
        schema: Bmi088SchemaFrame,
        raw_frame: Vec<u8>,
        parser: UiParserMeta,
    },
    TelemetryIdentity {
        timestamp_ms: u64,
        source: UiSource,
        identity: Bmi088IdentityFrame,
        raw_frame: Vec<u8>,
        parser: UiParserMeta,
    },
    TelemetrySample {
        timestamp_ms: u64,
        source: UiSource,
        sample: Bmi088SampleFrame,
        raw_frame: Vec<u8>,
        parser: UiParserMeta,
    },
    MavlinkPacket {
        timestamp_ms: u64,
        source: UiSource,
        packet: UiMavlinkPacket,
    },
    CrtpPacket {
        timestamp_ms: u64,
        source: UiSource,
        packet: UiCrtpPacket,
    },
    Capability {
        timestamp_ms: u64,
        source: UiSource,
        event: CapabilityEvent,
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
    SelfDescribingIdentity {
        timestamp_ms: u64,
        source: UiSource,
        identity: SelfDescribingIdentity,
    },
    SelfDescribingVariableCatalog {
        timestamp_ms: u64,
        source: UiSource,
        catalog: VariableCatalogPage,
    },
    SelfDescribingCommandCatalog {
        timestamp_ms: u64,
        source: UiSource,
        catalog: CommandCatalogPage,
    },
    SelfDescribingSample {
        timestamp_ms: u64,
        source: UiSource,
        sample: SelfDescribingSample,
    },
    SelfDescribingSetVariable {
        timestamp_ms: u64,
        source: UiSource,
        set_variable: SetVariable,
    },
    SelfDescribingAckResult {
        timestamp_ms: u64,
        source: UiSource,
        result: AckResult,
    },
    SelfDescribingVerdict {
        timestamp_ms: u64,
        source: UiSource,
        verdict: SelfDescribingStreamingDriftVerdict,
    },
    ProtocolDetected {
        timestamp_ms: u64,
        source: UiSource,
        protocol: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMavlinkPacket {
    pub sequence: u8,
    pub system_id: u8,
    pub component_id: u8,
    pub message_id: u32,
    pub payload_len: usize,
    pub fields: std::collections::BTreeMap<String, String>,
}

impl From<MavlinkPacket> for UiMavlinkPacket {
    fn from(value: MavlinkPacket) -> Self {
        let fields = value.fields();
        Self {
            sequence: value.sequence,
            system_id: value.system_id,
            component_id: value.component_id,
            message_id: value.message_id,
            payload_len: value.payload.len(),
            fields,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCrtpPacket {
    pub port: String,
    pub channel: u8,
    pub payload_len: usize,
    pub fields: std::collections::BTreeMap<String, String>,
}

impl From<CrtpPacket> for UiCrtpPacket {
    fn from(value: CrtpPacket) -> Self {
        let fields = value.fields();
        Self {
            port: value.port.label().to_string(),
            channel: value.channel,
            payload_len: value.payload.len(),
            fields,
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
                    raw: line.payload.raw,
                },
                parser: parser.into(),
            },
            MessageKind::ShellOutput(line) => Self::ShellOutput {
                timestamp_ms,
                source,
                line: UiLinePayload {
                    direction: line.direction.into(),
                    raw_length: line.payload.raw.len(),
                    text: line.payload.text,
                    raw: line.payload.raw,
                },
                parser: parser.into(),
            },
            MessageKind::TelemetrySchema(schema) => Self::TelemetrySchema {
                timestamp_ms,
                source,
                raw_frame: encode_schema_frame(&schema),
                schema,
                parser: parser.into(),
            },
            MessageKind::TelemetryIdentity(identity) => Self::TelemetryIdentity {
                timestamp_ms,
                source,
                raw_frame: encode_identity_frame(&identity),
                identity,
                parser: parser.into(),
            },
            MessageKind::TelemetrySample(sample) => Self::TelemetrySample {
                timestamp_ms,
                source,
                raw_frame: encode_sample_frame(&sample),
                sample,
                parser: parser.into(),
            },
            MessageKind::MavlinkPacket(packet) => Self::MavlinkPacket {
                timestamp_ms,
                source,
                packet: packet.into(),
            },
            MessageKind::CrtpPacket(packet) => Self::CrtpPacket {
                timestamp_ms,
                source,
                packet: packet.into(),
            },
            MessageKind::Capability(event) => Self::Capability {
                timestamp_ms,
                source,
                event,
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
            MessageKind::SelfDescribingIdentity(identity) => Self::SelfDescribingIdentity {
                timestamp_ms,
                source,
                identity,
            },
            MessageKind::SelfDescribingVariableCatalog(catalog) => {
                Self::SelfDescribingVariableCatalog {
                    timestamp_ms,
                    source,
                    catalog,
                }
            }
            MessageKind::SelfDescribingCommandCatalog(catalog) => {
                Self::SelfDescribingCommandCatalog {
                    timestamp_ms,
                    source,
                    catalog,
                }
            }
            MessageKind::SelfDescribingSample(sample) => Self::SelfDescribingSample {
                timestamp_ms,
                source,
                sample,
            },
            MessageKind::SelfDescribingSetVariable(set_var) => Self::SelfDescribingSetVariable {
                timestamp_ms,
                source,
                set_variable: set_var,
            },
            MessageKind::SelfDescribingAckResult(result) => Self::SelfDescribingAckResult {
                timestamp_ms,
                source,
                result,
            },
            MessageKind::SelfDescribingVerdict(verdict) => Self::SelfDescribingVerdict {
                timestamp_ms,
                source,
                verdict,
            },
            MessageKind::ProtocolDetected(event) => Self::ProtocolDetected {
                timestamp_ms,
                source,
                protocol: event.protocol,
            },
        }
    }
}

fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
