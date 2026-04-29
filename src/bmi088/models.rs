/// BMI088 protocol data types and frame models.

use crate::message::LinePayload;
use crate::serial::FramedLine;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088FieldDescriptor {
    pub field_id: u8,
    pub field_type: u8,
    pub name: String,
    pub unit: String,
    pub scale_q: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088SchemaFrame {
    pub seq: u8,
    pub schema_version: u8,
    pub rate_hz: u32,
    pub sample_len: usize,
    pub fields: Vec<Bmi088FieldDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088IdentityFrame {
    pub seq: u8,
    pub identity_format_version: u8,
    pub device_name: String,
    pub board_name: String,
    pub firmware_version: String,
    pub protocol_name: String,
    pub protocol_version: String,
    pub transport_name: String,
    pub sample_rate_hz: u16,
    pub schema_field_count: u8,
    pub sample_payload_len: u8,
    pub protocol_version_byte: u8,
    pub feature_flags: u16,
    pub baud_rate: u32,
    pub protocol_minor_version: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088SampleField {
    pub name: String,
    pub raw: i16,
    pub value: f64,
    pub unit: Option<String>,
    pub scale_q: i8,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088SampleFrame {
    pub seq: u8,
    pub fields: Vec<Bmi088SampleField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bmi088HostCommand {
    Ack,
    Start,
    Stop,
    ReqSchema,
    ReqIdentity,
    ReqTuning,
    SetTuning,
    ShellExec,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bmi088Frame {
    Identity(Bmi088IdentityFrame),
    Schema(Bmi088SchemaFrame),
    Sample(Bmi088SampleFrame),
    ShellOutput(LinePayload),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TelemetryPacket {
    Text(FramedLine),
    ShellOutput(LinePayload),
    Identity(Bmi088IdentityFrame),
    Schema(Bmi088SchemaFrame),
    Sample(Bmi088SampleFrame),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bmi088SessionPhase {
    AwaitingSchema,
    AwaitingAck,
    AwaitingStart,
    Streaming,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bmi088DecodeError {
    InvalidSof,
    InvalidVersion,
    InvalidCrc,
    SchemaMismatch(String),
    MalformedFrame(String),
}

impl std::fmt::Display for Bmi088DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSof => write!(formatter, "invalid BMI088 SOF"),
            Self::InvalidVersion => write!(formatter, "invalid BMI088 version"),
            Self::InvalidCrc => write!(formatter, "invalid BMI088 CRC"),
            Self::SchemaMismatch(message) => write!(formatter, "schema mismatch: {message}"),
            Self::MalformedFrame(message) => write!(formatter, "malformed BMI088 frame: {message}"),
        }
    }
}

impl std::error::Error for Bmi088DecodeError {}
