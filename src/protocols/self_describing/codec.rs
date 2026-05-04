/// Codec for encoding and decoding self-describing protocol frames.
///
/// This module handles the binary serialization/deserialization of protocol frames.
/// The wire format uses a simple TLV-like structure with a fixed header.
use super::frame::*;
use std::io;

/// Frame type identifiers for the wire format.
const FRAME_TYPE_IDENTITY: u8 = 0x01;
const FRAME_TYPE_VARIABLE_CATALOG_PAGE: u8 = 0x02;
const FRAME_TYPE_COMMAND_CATALOG_PAGE: u8 = 0x03;
const FRAME_TYPE_HOST_ACK: u8 = 0x04;
const FRAME_TYPE_TELEMETRY_SAMPLE: u8 = 0x05;
const FRAME_TYPE_SET_VARIABLE: u8 = 0x06;
const FRAME_TYPE_ACK_RESULT: u8 = 0x07;

/// Value type identifiers for the wire format.
const VALUE_TYPE_U8: u8 = 0x01;
const VALUE_TYPE_I8: u8 = 0x02;
const VALUE_TYPE_U16: u8 = 0x03;
const VALUE_TYPE_I16: u8 = 0x04;
const VALUE_TYPE_U32: u8 = 0x05;
const VALUE_TYPE_I32: u8 = 0x06;
const VALUE_TYPE_F32: u8 = 0x07;

/// Ack stage identifiers for the wire format.
const ACK_STAGE_IDENTITY: u8 = 0x01;
const ACK_STAGE_COMMAND_CATALOG: u8 = 0x02;
const ACK_STAGE_VARIABLE_CATALOG: u8 = 0x03;
const ACK_STAGE_STREAMING: u8 = 0x04;

/// Decode error types.
#[derive(Debug)]
pub enum DecodeError {
    Io(io::Error),
    InvalidFrameType(u8),
    InvalidValueType(u8),
    InvalidAckStage(u8),
    TruncatedData,
    InvalidUtf8,
}

impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::InvalidFrameType(t) => write!(f, "invalid frame type: {t}"),
            Self::InvalidValueType(t) => write!(f, "invalid value type: {t}"),
            Self::InvalidAckStage(s) => write!(f, "invalid ack stage: {s}"),
            Self::TruncatedData => write!(f, "truncated data"),
            Self::InvalidUtf8 => write!(f, "invalid utf-8"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Encode a frame to bytes.
pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let mut buf = Vec::new();
    match frame {
        Frame::Identity(identity) => {
            buf.push(FRAME_TYPE_IDENTITY);
            encode_identity(identity, &mut buf);
        }
        Frame::VariableCatalogPage(page) => {
            buf.push(FRAME_TYPE_VARIABLE_CATALOG_PAGE);
            encode_variable_catalog_page(page, &mut buf);
        }
        Frame::CommandCatalogPage(page) => {
            buf.push(FRAME_TYPE_COMMAND_CATALOG_PAGE);
            encode_command_catalog_page(page, &mut buf);
        }
        Frame::HostAck(ack) => {
            buf.push(FRAME_TYPE_HOST_ACK);
            encode_host_ack(ack, &mut buf);
        }
        Frame::TelemetrySample(sample) => {
            buf.push(FRAME_TYPE_TELEMETRY_SAMPLE);
            encode_telemetry_sample(sample, &mut buf);
        }
        Frame::SetVariable(set_var) => {
            buf.push(FRAME_TYPE_SET_VARIABLE);
            encode_set_variable(set_var, &mut buf);
        }
        Frame::AckResult(result) => {
            buf.push(FRAME_TYPE_ACK_RESULT);
            encode_ack_result(result, &mut buf);
        }
    }
    buf
}

/// Decode a frame from bytes.
pub fn decode_frame(data: &[u8]) -> Result<Frame, DecodeError> {
    if data.is_empty() {
        return Err(DecodeError::TruncatedData);
    }

    let frame_type = data[0];
    let payload = &data[1..];

    match frame_type {
        FRAME_TYPE_IDENTITY => Ok(Frame::Identity(decode_identity(payload)?)),
        FRAME_TYPE_VARIABLE_CATALOG_PAGE => Ok(Frame::VariableCatalogPage(
            decode_variable_catalog_page(payload)?,
        )),
        FRAME_TYPE_COMMAND_CATALOG_PAGE => Ok(Frame::CommandCatalogPage(
            decode_command_catalog_page(payload)?,
        )),
        FRAME_TYPE_HOST_ACK => Ok(Frame::HostAck(decode_host_ack(payload)?)),
        FRAME_TYPE_TELEMETRY_SAMPLE => {
            Ok(Frame::TelemetrySample(decode_telemetry_sample(payload)?))
        }
        FRAME_TYPE_SET_VARIABLE => Ok(Frame::SetVariable(decode_set_variable(payload)?)),
        FRAME_TYPE_ACK_RESULT => Ok(Frame::AckResult(decode_ack_result(payload)?)),
        _ => Err(DecodeError::InvalidFrameType(frame_type)),
    }
}

fn encode_identity(identity: &Identity, buf: &mut Vec<u8>) {
    buf.push(identity.protocol_version);
    encode_string(&identity.device_name, buf);
    encode_string(&identity.firmware_version, buf);
    buf.extend_from_slice(&identity.sample_rate_hz.to_le_bytes());
    buf.extend_from_slice(&identity.variable_count.to_le_bytes());
    buf.extend_from_slice(&identity.command_count.to_le_bytes());
    buf.extend_from_slice(&identity.sample_payload_len.to_le_bytes());
}

fn decode_identity(data: &[u8]) -> Result<Identity, DecodeError> {
    let mut cursor = 0;
    if data.is_empty() {
        return Err(DecodeError::TruncatedData);
    }
    let protocol_version = data[0];
    cursor += 1;

    let (device_name, bytes_read) = decode_identity_string(data, cursor)?;
    cursor += bytes_read;

    let (firmware_version, bytes_read) = decode_identity_string(data, cursor)?;
    cursor += bytes_read;

    if data.len() < cursor + 10 {
        return Err(DecodeError::TruncatedData);
    }

    let sample_rate_hz = u32::from_le_bytes([
        data[cursor],
        data[cursor + 1],
        data[cursor + 2],
        data[cursor + 3],
    ]);
    cursor += 4;

    let variable_count = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
    cursor += 2;

    let command_count = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
    cursor += 2;

    let sample_payload_len = u16::from_le_bytes([data[cursor], data[cursor + 1]]);

    Ok(Identity {
        protocol_version,
        device_name,
        firmware_version,
        sample_rate_hz,
        variable_count,
        command_count,
        sample_payload_len,
    })
}

fn decode_identity_string(data: &[u8], offset: usize) -> Result<(String, usize), DecodeError> {
    match decode_string(data, offset) {
        Ok(value) => Ok(value),
        Err(DecodeError::TruncatedData) => decode_short_string(data, offset),
        Err(err) => Err(err),
    }
}

fn encode_variable_catalog_page(page: &VariableCatalogPage, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&page.page.to_le_bytes());
    buf.extend_from_slice(&page.total_pages.to_le_bytes());
    buf.push(page.variables.len() as u8);
    for var in &page.variables {
        encode_string(&var.name, buf);
        buf.extend_from_slice(&var.order.to_le_bytes());
        encode_string(&var.unit, buf);
        buf.push(var.adjustable as u8);
        buf.push(value_type_to_u8(var.value_type));
    }
}

fn decode_variable_catalog_page(data: &[u8]) -> Result<VariableCatalogPage, DecodeError> {
    decode_variable_catalog_page_with_mode(data, CatalogStringMode::Canonical)
        .or_else(|_| decode_variable_catalog_page_with_mode(data, CatalogStringMode::Short))
}

fn encode_command_catalog_page(page: &CommandCatalogPage, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&page.page.to_le_bytes());
    buf.extend_from_slice(&page.total_pages.to_le_bytes());
    buf.push(page.commands.len() as u8);
    for cmd in &page.commands {
        encode_string(&cmd.id, buf);
        encode_string(&cmd.params, buf);
        encode_string(&cmd.docs, buf);
    }
}

fn decode_command_catalog_page(data: &[u8]) -> Result<CommandCatalogPage, DecodeError> {
    decode_command_catalog_page_with_mode(data, CatalogStringMode::Canonical)
        .or_else(|_| decode_command_catalog_page_with_mode(data, CatalogStringMode::Short))
}

fn encode_host_ack(ack: &HostAck, buf: &mut Vec<u8>) {
    buf.push(ack_stage_to_u8(ack.stage));
}

fn decode_host_ack(data: &[u8]) -> Result<HostAck, DecodeError> {
    if data.len() < 1 {
        return Err(DecodeError::TruncatedData);
    }

    let stage = decode_ack_stage(data[0])?;
    Ok(HostAck { stage })
}

fn encode_telemetry_sample(sample: &TelemetrySample, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&sample.seq.to_le_bytes());
    buf.extend_from_slice(&(sample.changed_bitmap.len() as u16).to_le_bytes());
    buf.extend_from_slice(&sample.changed_bitmap);
    buf.extend_from_slice(&sample.values);
}

fn decode_telemetry_sample(data: &[u8]) -> Result<TelemetrySample, DecodeError> {
    if data.len() < 6 {
        return Err(DecodeError::TruncatedData);
    }

    let seq = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let bitmap_len = u16::from_le_bytes([data[4], data[5]]) as usize;

    if data.len() < 6 + bitmap_len {
        return Err(DecodeError::TruncatedData);
    }

    let changed_bitmap = data[6..6 + bitmap_len].to_vec();
    let values = data[6 + bitmap_len..].to_vec();

    Ok(TelemetrySample {
        seq,
        changed_bitmap,
        values,
    })
}

fn encode_set_variable(set_var: &SetVariable, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&set_var.seq.to_le_bytes());
    buf.extend_from_slice(&set_var.variable_index.to_le_bytes());
    buf.extend_from_slice(&(set_var.value.len() as u16).to_le_bytes());
    buf.extend_from_slice(&set_var.value);
}

fn decode_set_variable(data: &[u8]) -> Result<SetVariable, DecodeError> {
    if data.len() < 8 {
        return Err(DecodeError::TruncatedData);
    }

    let seq = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let variable_index = u16::from_le_bytes([data[4], data[5]]);
    let value_len = u16::from_le_bytes([data[6], data[7]]) as usize;

    if data.len() < 8 + value_len {
        return Err(DecodeError::TruncatedData);
    }

    let value = data[8..8 + value_len].to_vec();

    Ok(SetVariable {
        seq,
        variable_index,
        value,
    })
}

fn encode_ack_result(result: &AckResult, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&result.seq.to_le_bytes());
    buf.push(result.code);
    encode_string(&result.message, buf);
}

fn decode_ack_result(data: &[u8]) -> Result<AckResult, DecodeError> {
    if data.len() < 5 {
        return Err(DecodeError::TruncatedData);
    }

    let seq = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let code = data[4];
    let (message, _) = decode_string(data, 5)?;

    Ok(AckResult { seq, code, message })
}

fn encode_string(s: &str, buf: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(bytes);
}

fn decode_string(data: &[u8], offset: usize) -> Result<(String, usize), DecodeError> {
    if data.len() < offset + 2 {
        return Err(DecodeError::TruncatedData);
    }

    let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let start = offset + 2;

    if data.len() < start + len {
        return Err(DecodeError::TruncatedData);
    }

    let s = std::str::from_utf8(&data[start..start + len]).map_err(|_| DecodeError::InvalidUtf8)?;

    Ok((s.to_string(), 2 + len))
}

#[derive(Clone, Copy)]
enum CatalogStringMode {
    Canonical,
    Short,
}

fn decode_variable_catalog_page_with_mode(
    data: &[u8],
    mode: CatalogStringMode,
) -> Result<VariableCatalogPage, DecodeError> {
    if data.len() < 5 {
        return Err(DecodeError::TruncatedData);
    }

    let page = u16::from_le_bytes([data[0], data[1]]);
    let total_pages = u16::from_le_bytes([data[2], data[3]]);
    let count = data[4] as usize;
    let mut cursor = 5;

    let mut variables = Vec::with_capacity(count);
    for _ in 0..count {
        let (name, bytes_read) = decode_catalog_string_with_mode(data, cursor, mode)?;
        cursor += bytes_read;

        if data.len() < cursor + 2 {
            return Err(DecodeError::TruncatedData);
        }
        let order = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
        cursor += 2;

        let (unit, bytes_read) = decode_catalog_string_with_mode(data, cursor, mode)?;
        cursor += bytes_read;

        if data.len() < cursor + 2 {
            return Err(DecodeError::TruncatedData);
        }
        let adjustable = data[cursor] != 0;
        cursor += 1;

        let value_type = decode_value_type(data[cursor])?;
        cursor += 1;

        variables.push(VariableDescriptor {
            name,
            order,
            unit,
            adjustable,
            value_type,
        });
    }

    Ok(VariableCatalogPage {
        page,
        total_pages,
        variables,
    })
}

fn decode_command_catalog_page_with_mode(
    data: &[u8],
    mode: CatalogStringMode,
) -> Result<CommandCatalogPage, DecodeError> {
    if data.len() < 5 {
        return Err(DecodeError::TruncatedData);
    }

    let page = u16::from_le_bytes([data[0], data[1]]);
    let total_pages = u16::from_le_bytes([data[2], data[3]]);
    let count = data[4] as usize;
    let mut cursor = 5;

    let mut commands = Vec::with_capacity(count);
    for _ in 0..count {
        let (id, bytes_read) = decode_catalog_string_with_mode(data, cursor, mode)?;
        cursor += bytes_read;

        let (params, bytes_read) = decode_catalog_string_with_mode(data, cursor, mode)?;
        cursor += bytes_read;

        let (docs, bytes_read) = decode_catalog_string_with_mode(data, cursor, mode)?;
        cursor += bytes_read;

        commands.push(CommandDescriptor { id, params, docs });
    }

    Ok(CommandCatalogPage {
        page,
        total_pages,
        commands,
    })
}

fn decode_catalog_string_with_mode(
    data: &[u8],
    offset: usize,
    mode: CatalogStringMode,
) -> Result<(String, usize), DecodeError> {
    match mode {
        CatalogStringMode::Canonical => decode_string(data, offset),
        CatalogStringMode::Short => decode_short_string(data, offset),
    }
}

fn decode_short_string(data: &[u8], offset: usize) -> Result<(String, usize), DecodeError> {
    if data.len() < offset + 1 {
        return Err(DecodeError::TruncatedData);
    }

    let len = data[offset] as usize;
    let start = offset + 1;

    if data.len() < start + len {
        return Err(DecodeError::TruncatedData);
    }

    let s = std::str::from_utf8(&data[start..start + len]).map_err(|_| DecodeError::InvalidUtf8)?;

    Ok((s.to_string(), 1 + len))
}

fn value_type_to_u8(vt: ValueType) -> u8 {
    match vt {
        ValueType::U8 => VALUE_TYPE_U8,
        ValueType::I8 => VALUE_TYPE_I8,
        ValueType::U16 => VALUE_TYPE_U16,
        ValueType::I16 => VALUE_TYPE_I16,
        ValueType::U32 => VALUE_TYPE_U32,
        ValueType::I32 => VALUE_TYPE_I32,
        ValueType::F32 => VALUE_TYPE_F32,
    }
}

fn decode_value_type(vt: u8) -> Result<ValueType, DecodeError> {
    match vt {
        VALUE_TYPE_U8 => Ok(ValueType::U8),
        VALUE_TYPE_I8 => Ok(ValueType::I8),
        VALUE_TYPE_U16 => Ok(ValueType::U16),
        VALUE_TYPE_I16 => Ok(ValueType::I16),
        VALUE_TYPE_U32 => Ok(ValueType::U32),
        VALUE_TYPE_I32 => Ok(ValueType::I32),
        VALUE_TYPE_F32 => Ok(ValueType::F32),
        _ => Err(DecodeError::InvalidValueType(vt)),
    }
}

fn ack_stage_to_u8(stage: AckStage) -> u8 {
    match stage {
        AckStage::Identity => ACK_STAGE_IDENTITY,
        AckStage::CommandCatalog => ACK_STAGE_COMMAND_CATALOG,
        AckStage::VariableCatalog => ACK_STAGE_VARIABLE_CATALOG,
        AckStage::Streaming => ACK_STAGE_STREAMING,
    }
}

fn decode_ack_stage(stage: u8) -> Result<AckStage, DecodeError> {
    match stage {
        ACK_STAGE_IDENTITY => Ok(AckStage::Identity),
        ACK_STAGE_COMMAND_CATALOG => Ok(AckStage::CommandCatalog),
        ACK_STAGE_VARIABLE_CATALOG => Ok(AckStage::VariableCatalog),
        ACK_STAGE_STREAMING => Ok(AckStage::Streaming),
        _ => Err(DecodeError::InvalidAckStage(stage)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_roundtrip() {
        let identity = Identity {
            protocol_version: 1,
            device_name: "Test Device".to_string(),
            firmware_version: "1.0.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 10,
            command_count: 5,
            sample_payload_len: 40,
        };

        let frame = Frame::Identity(identity.clone());
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode identity");

        match decoded {
            Frame::Identity(d) => {
                assert_eq!(d.protocol_version, 1);
                assert_eq!(d.device_name, "Test Device");
                assert_eq!(d.firmware_version, "1.0.0");
                assert_eq!(d.sample_rate_hz, 100);
                assert_eq!(d.variable_count, 10);
                assert_eq!(d.command_count, 5);
                assert_eq!(d.sample_payload_len, 40);
            }
            _ => panic!("expected identity frame"),
        }
    }

    #[test]
    fn test_identity_compatibility_with_short_string_lengths() {
        let encoded = vec![
            0x01, 0x01, 0x06, b'C', b'Y', b'T', b'4', b'B', b'B', 0x03, b'0', b'.', b'1',
            0x64, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x04, 0x00, 0x3C, 0x00,
        ];

        let decoded = decode_frame(&encoded).expect("decode identity short-string payload");

        match decoded {
            Frame::Identity(identity) => {
                assert_eq!(identity.protocol_version, 1);
                assert_eq!(identity.device_name, "CYT4BB");
                assert_eq!(identity.firmware_version, "0.1");
                assert_eq!(identity.sample_rate_hz, 100);
                assert_eq!(identity.variable_count, 30);
                assert_eq!(identity.command_count, 4);
                assert_eq!(identity.sample_payload_len, 60);
            }
            _ => panic!("expected identity frame"),
        }
    }

    #[test]
    fn test_variable_catalog_page_roundtrip() {
        let page = VariableCatalogPage {
            page: 0,
            total_pages: 2,
            variables: vec![
                VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
                VariableDescriptor {
                    name: "pid_gain".to_string(),
                    order: 1,
                    unit: "".to_string(),
                    adjustable: true,
                    value_type: ValueType::F32,
                },
            ],
        };

        let frame = Frame::VariableCatalogPage(page);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode variable catalog page");

        match decoded {
            Frame::VariableCatalogPage(d) => {
                assert_eq!(d.page, 0);
                assert_eq!(d.total_pages, 2);
                assert_eq!(d.variables.len(), 2);
                assert_eq!(d.variables[0].name, "acc_x");
                assert_eq!(d.variables[0].value_type, ValueType::I16);
                assert!(!d.variables[0].adjustable);
                assert_eq!(d.variables[1].name, "pid_gain");
                assert!(d.variables[1].adjustable);
            }
            _ => panic!("expected variable catalog page frame"),
        }
    }

    #[test]
    fn test_command_catalog_page_roundtrip() {
        let page = CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "none".to_string(),
                docs: "Start streaming".to_string(),
            }],
        };

        let frame = Frame::CommandCatalogPage(page);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode command catalog page");

        match decoded {
            Frame::CommandCatalogPage(d) => {
                assert_eq!(d.page, 0);
                assert_eq!(d.total_pages, 1);
                assert_eq!(d.commands.len(), 1);
                assert_eq!(d.commands[0].id, "start");
                assert_eq!(d.commands[0].docs, "Start streaming");
            }
            _ => panic!("expected command catalog page frame"),
        }
    }

    #[test]
    fn test_host_ack_roundtrip() {
        let ack = HostAck {
            stage: AckStage::VariableCatalog,
        };

        let frame = Frame::HostAck(ack);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode host ack");

        match decoded {
            Frame::HostAck(d) => {
                assert_eq!(d.stage, AckStage::VariableCatalog);
            }
            _ => panic!("expected host ack frame"),
        }
    }

    #[test]
    fn test_host_ack_is_compact_two_bytes_on_wire() {
        let ack = HostAck {
            stage: AckStage::Identity,
        };

        let encoded = encode_frame(&Frame::HostAck(ack));

        assert_eq!(encoded, vec![FRAME_TYPE_HOST_ACK, ACK_STAGE_IDENTITY]);
        assert_eq!(encoded.len(), 2);
    }

    #[test]
    fn test_telemetry_sample_roundtrip() {
        let sample = TelemetrySample {
            seq: 42,
            changed_bitmap: vec![0b10101010, 0b00000001],
            values: vec![1, 2, 3, 4],
        };

        let frame = Frame::TelemetrySample(sample);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode telemetry sample");

        match decoded {
            Frame::TelemetrySample(d) => {
                assert_eq!(d.seq, 42);
                assert_eq!(d.changed_bitmap, vec![0b10101010, 0b00000001]);
                assert_eq!(d.values, vec![1, 2, 3, 4]);
            }
            _ => panic!("expected telemetry sample frame"),
        }
    }

    #[test]
    fn test_set_variable_roundtrip() {
        let set_var = SetVariable {
            seq: 10,
            variable_index: 5,
            value: vec![0x00, 0x01, 0x02, 0x03],
        };

        let frame = Frame::SetVariable(set_var);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode set variable");

        match decoded {
            Frame::SetVariable(d) => {
                assert_eq!(d.seq, 10);
                assert_eq!(d.variable_index, 5);
                assert_eq!(d.value, vec![0x00, 0x01, 0x02, 0x03]);
            }
            _ => panic!("expected set variable frame"),
        }
    }

    #[test]
    fn test_ack_result_roundtrip() {
        let result = AckResult {
            seq: 10,
            code: 0,
            message: "success".to_string(),
        };

        let frame = Frame::AckResult(result);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode ack result");

        match decoded {
            Frame::AckResult(d) => {
                assert_eq!(d.seq, 10);
                assert_eq!(d.code, 0);
                assert_eq!(d.message, "success");
            }
            _ => panic!("expected ack result frame"),
        }
    }

    #[test]
    fn test_decode_truncated_data() {
        let data = [0x01]; // Identity frame type but no payload
        assert!(decode_frame(&data).is_err());
    }

    #[test]
    fn test_decode_invalid_frame_type() {
        let data = [0xFF];
        assert!(matches!(
            decode_frame(&data),
            Err(DecodeError::InvalidFrameType(0xFF))
        ));
    }
}
