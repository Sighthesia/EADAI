/// Frame types for the generic self-describing device protocol.
///
/// This module defines the semantic frame types that can be exchanged
/// between device and host during handshake and streaming phases.
use serde::{Deserialize, Serialize};

/// Protocol version for the self-describing protocol.
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum number of variables per catalog page.
pub const MAX_VARIABLES_PER_PAGE: usize = 32;

/// Maximum number of commands per catalog page.
pub const MAX_COMMANDS_PER_PAGE: usize = 16;

/// Variable value encoding types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
}

impl ValueType {
    /// Returns the byte size of this value type.
    pub fn byte_size(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
        }
    }
}

/// Variable descriptor from the device's variable catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableDescriptor {
    /// Variable name (UTF-8).
    pub name: String,
    /// Order index in the sample frame (0-based).
    pub order: u16,
    /// Unit string (UTF-8).
    pub unit: String,
    /// Whether this variable is adjustable by the host.
    pub adjustable: bool,
    /// Value encoding type.
    pub value_type: ValueType,
}

/// Command descriptor from the device's command catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    /// Command identifier (UTF-8).
    pub id: String,
    /// Parameter description (UTF-8).
    pub params: String,
    /// Documentation text (UTF-8).
    pub docs: String,
}

/// Device identity frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Protocol version.
    pub protocol_version: u8,
    /// Device name.
    pub device_name: String,
    /// Firmware version.
    pub firmware_version: String,
    /// Sample rate in Hz.
    pub sample_rate_hz: u32,
    /// Number of variables in the catalog.
    pub variable_count: u16,
    /// Number of commands in the catalog.
    pub command_count: u16,
    /// Total sample payload length in bytes.
    pub sample_payload_len: u16,
}

/// Variable catalog page (may be part of a multi-page transfer).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableCatalogPage {
    /// Page index (0-based).
    pub page: u16,
    /// Total number of pages.
    pub total_pages: u16,
    /// Variables in this page.
    pub variables: Vec<VariableDescriptor>,
}

/// Command catalog page (may be part of a multi-page transfer).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCatalogPage {
    /// Page index (0-based).
    pub page: u16,
    /// Total number of pages.
    pub total_pages: u16,
    /// Commands in this page.
    pub commands: Vec<CommandDescriptor>,
}

/// Host acknowledgment frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostAck {
    /// Stage being acknowledged.
    pub stage: AckStage,
}

/// Stages that can be acknowledged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckStage {
    Identity,
    CommandCatalog,
    VariableCatalog,
    Streaming,
}

/// Unified acknowledgment/result frame for variable writes and command execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AckResult {
    /// Sequence number of the request being acknowledged.
    pub seq: u32,
    /// Result code (0 = success, non-zero = error).
    pub code: u8,
    /// Optional message.
    pub message: String,
}

/// Telemetry sample frame with bitmap compression.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetrySample {
    /// Sequence number (monotonically increasing).
    pub seq: u32,
    /// Bitmap indicating which variables have changed (bit i = 1 means changed).
    pub changed_bitmap: Vec<u8>,
    /// Raw variable values (only for changed variables).
    pub values: Vec<u8>,
}

/// Set variable command from host to device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetVariable {
    /// Sequence number for request/response matching.
    pub seq: u32,
    /// Variable index (0-based).
    pub variable_index: u16,
    /// Raw value bytes (length must match variable's value type).
    pub value: Vec<u8>,
}

/// Semantic frame types that can be decoded from raw bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frame {
    /// Device identity.
    Identity(Identity),
    /// Variable catalog page.
    VariableCatalogPage(VariableCatalogPage),
    /// Command catalog page.
    CommandCatalogPage(CommandCatalogPage),
    /// Host acknowledgment.
    HostAck(HostAck),
    /// Telemetry sample.
    TelemetrySample(TelemetrySample),
    /// Set variable command.
    SetVariable(SetVariable),
    /// Acknowledgment/result.
    AckResult(AckResult),
}
