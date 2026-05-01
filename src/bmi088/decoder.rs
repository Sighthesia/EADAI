/// BMI088 frame decoding: identity TLV, schema payload (modern + legacy), sample payload,
/// binary frame dispatch, and the stream decoder.
use super::constants::*;
use super::encoder::{compact_field_name, compact_unit_name, crc16_ccitt, default_schema};
use super::models::{
    Bmi088DecodeError, Bmi088FieldDescriptor, Bmi088Frame, Bmi088IdentityFrame, Bmi088SampleFrame,
    Bmi088SchemaFrame, TelemetryPacket,
};
use crate::message::LinePayload;
use crate::serial::LineFramer;

// ── Stream decoder ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Bmi088StreamDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
    text_framer: LineFramer,
    schema: Option<Bmi088SchemaFrame>,
    identity: Option<Bmi088IdentityFrame>,
}

impl Default for Bmi088StreamDecoder {
    fn default() -> Self {
        Self::new(crate::cli::DEFAULT_MAX_FRAME_BYTES)
    }
}

impl Bmi088StreamDecoder {
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
            text_framer: LineFramer::new(),
            schema: None,
            identity: None,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<TelemetryPacket> {
        if !chunk.is_empty() {
            eprintln!(
                "[bmi088][stream] rx chunk bytes={} preview={}",
                chunk.len(),
                hex_preview(chunk, 16),
            );
        }
        self.buffer.extend_from_slice(chunk);
        let mut packets = Vec::new();

        loop {
            if let Some(sof_index) = find_sof(&self.buffer) {
                if sof_index > 0 {
                    let prefix = self.buffer.drain(..sof_index).collect::<Vec<_>>();
                    packets.extend(
                        self.text_framer
                            .push(&prefix)
                            .into_iter()
                            .map(TelemetryPacket::Text),
                    );
                    continue;
                }

                if self.buffer.len() < BMI088_MIN_FRAME_LEN {
                    break;
                }

                let payload_len = self.buffer[6] as usize;
                let frame_len = BMI088_HEADER_LEN + payload_len + BMI088_CRC_LEN;
                if frame_len > self.max_buffer_bytes {
                    eprintln!(
                        "[bmi088][stream] drop oversize frame payload_len={} frame_len={} max_buffer_bytes={}",
                        payload_len, frame_len, self.max_buffer_bytes,
                    );
                    self.buffer.drain(..1);
                    continue;
                }

                if self.buffer.len() < frame_len {
                    break;
                }

                let frame = self.buffer[..frame_len].to_vec();
                match decode_binary_frame_with_schema(&frame, self.schema.as_ref()) {
                    Ok(Bmi088Frame::Identity(identity)) => {
                        eprintln!(
                            "[bmi088][stream] decoded IDENTITY seq={} payload_len={} frame_len={}",
                            identity.seq, payload_len, frame_len,
                        );
                        self.buffer.drain(..frame_len);
                        self.identity = Some(identity.clone());
                        packets.push(TelemetryPacket::Identity(identity));
                    }
                    Ok(Bmi088Frame::Schema(schema)) => {
                        eprintln!(
                            "[bmi088][stream] decoded SCHEMA seq={} payload_len={} fields={} sample_len={}",
                            schema.seq,
                            payload_len,
                            schema.fields.len(),
                            schema.sample_len,
                        );
                        self.buffer.drain(..frame_len);
                        if self.should_cache_schema(&schema) {
                            self.schema = Some(schema.clone());
                        }
                        packets.push(TelemetryPacket::Schema(schema));
                    }
                    Ok(Bmi088Frame::Sample(sample)) => {
                        eprintln!(
                            "[bmi088][stream] decoded SAMPLE seq={} payload_len={} fields={} schema_cached={}",
                            sample.seq,
                            payload_len,
                            sample.fields.len(),
                            self.schema.is_some(),
                        );
                        self.buffer.drain(..frame_len);
                        packets.push(TelemetryPacket::Sample(sample));
                    }
                    Ok(Bmi088Frame::ShellOutput(output)) => {
                        eprintln!(
                            "[bmi088][stream] decoded SHELL_OUTPUT payload_len={} text_preview={}",
                            payload_len,
                            text_preview(&output.text, 80),
                        );
                        self.buffer.drain(..frame_len);
                        packets.push(TelemetryPacket::ShellOutput(output));
                    }
                    Err(
                        Bmi088DecodeError::InvalidCrc
                        | Bmi088DecodeError::InvalidVersion
                        | Bmi088DecodeError::InvalidSof,
                    ) => {
                        if let Err(error) =
                            decode_binary_frame_with_schema(&frame, self.schema.as_ref())
                        {
                            eprintln!(
                                "[bmi088][stream] frame decode error={} frame_len={} preview={}",
                                decode_error_label(&error),
                                frame_len,
                                hex_preview(&frame, 16),
                            );
                        }
                        self.buffer.drain(..1);
                    }
                    Err(Bmi088DecodeError::SchemaMismatch(_))
                    | Err(Bmi088DecodeError::MalformedFrame(_)) => {
                        if let Err(error) =
                            decode_binary_frame_with_schema(&frame, self.schema.as_ref())
                        {
                            eprintln!(
                                "[bmi088][stream] frame decode error={} frame_len={} preview={}",
                                decode_error_label(&error),
                                frame_len,
                                hex_preview(&frame, 16),
                            );
                        }
                        self.buffer.drain(..1);
                    }
                }
                continue;
            }

            if let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
                let prefix = self.buffer.drain(..=newline).collect::<Vec<_>>();
                packets.extend(
                    self.text_framer
                        .push(&prefix)
                        .into_iter()
                        .map(TelemetryPacket::Text),
                );
                continue;
            }

            if self.buffer.len() > self.max_buffer_bytes {
                let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes);
                self.buffer.drain(..drain.max(1));
            }

            break;
        }

        packets
    }

    fn should_cache_schema(&self, schema: &Bmi088SchemaFrame) -> bool {
        if let Some(identity) = &self.identity {
            let matches_identity_shape = schema.sample_len
                == usize::from(identity.sample_payload_len)
                && schema.fields.len() == usize::from(identity.schema_field_count);
            if !matches_identity_shape {
                eprintln!(
                    "[bmi088][stream] schema cache skipped seq={} field_count={} sample_len={} identity_field_count={} identity_sample_len={}",
                    schema.seq,
                    schema.fields.len(),
                    schema.sample_len,
                    identity.schema_field_count,
                    identity.sample_payload_len,
                );
            }
            return matches_identity_shape;
        }

        true
    }
}

// ── Binary frame decode ──────────────────────────────────────────────────────

pub fn decode_binary_frame(frame: &[u8]) -> Result<Bmi088Frame, Bmi088DecodeError> {
    decode_binary_frame_with_schema(frame, None)
}

pub fn decode_binary_frame_with_schema(
    frame: &[u8],
    schema: Option<&Bmi088SchemaFrame>,
) -> Result<Bmi088Frame, Bmi088DecodeError> {
    if frame.len() < BMI088_MIN_FRAME_LEN {
        return Err(Bmi088DecodeError::MalformedFrame(
            "frame too short".to_string(),
        ));
    }

    if frame[0..2] != BMI088_SOF {
        return Err(Bmi088DecodeError::InvalidSof);
    }
    if frame[2] != BMI088_VERSION {
        return Err(Bmi088DecodeError::InvalidVersion);
    }

    let payload_len = frame[6] as usize;
    let expected_len = BMI088_HEADER_LEN + payload_len + BMI088_CRC_LEN;
    if frame.len() != expected_len {
        return Err(Bmi088DecodeError::MalformedFrame(
            "length mismatch".to_string(),
        ));
    }

    let crc = u16::from_le_bytes([frame[expected_len - 2], frame[expected_len - 1]]);
    let calculated_crc = crc16_ccitt(&frame[..expected_len - 2]);
    if crc != calculated_crc {
        return Err(Bmi088DecodeError::InvalidCrc);
    }

    let frame_type = frame[3];
    let command = frame[4];
    let seq = frame[5];
    let payload = &frame[BMI088_HEADER_LEN..expected_len - 2];

    match (frame_type, command) {
        (BMI088_FRAME_TYPE_EVENT | BMI088_FRAME_TYPE_RESPONSE, BMI088_CMD_IDENTITY) => Ok(
            Bmi088Frame::Identity(decode_identity_payload_with_seq(seq, payload)?),
        ),
        (BMI088_FRAME_TYPE_EVENT | BMI088_FRAME_TYPE_RESPONSE, BMI088_CMD_SCHEMA) => Ok(
            Bmi088Frame::Schema(decode_schema_payload_with_seq(seq, payload)?),
        ),
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SHELL_OUTPUT) => {
            Ok(Bmi088Frame::ShellOutput(LinePayload {
                text: String::from_utf8_lossy(payload).into_owned(),
                raw: payload.to_vec(),
            }))
        }
        (BMI088_FRAME_TYPE_EVENT | BMI088_FRAME_TYPE_RESPONSE, BMI088_CMD_SAMPLE) => {
            let fallback_schema = default_schema();
            let active_schema = schema.unwrap_or(&fallback_schema);
            match decode_sample_payload_with_schema_and_seq(payload, active_schema, seq) {
                Ok(sample) => Ok(Bmi088Frame::Sample(sample)),
                Err(Bmi088DecodeError::SchemaMismatch(_)) if schema.is_some() => {
                    eprintln!(
                        "[bmi088][sample] active schema mismatch seq={} active_field_count={} active_sample_len={} fallback_field_count={} fallback_sample_len={}",
                        seq,
                        active_schema.fields.len(),
                        active_schema.sample_len,
                        fallback_schema.fields.len(),
                        fallback_schema.sample_len,
                    );
                    Ok(Bmi088Frame::Sample(
                        decode_sample_payload_with_schema_and_seq(payload, &fallback_schema, seq)?,
                    ))
                }
                Err(error) => Err(error),
            }
        }
        _ => Err(Bmi088DecodeError::MalformedFrame(
            "unsupported command".to_string(),
        )),
    }
}

// ── Identity TLV decode ──────────────────────────────────────────────────────

pub fn decode_identity_payload(payload: &[u8]) -> Result<Bmi088IdentityFrame, Bmi088DecodeError> {
    decode_identity_payload_with_seq(0, payload)
}

pub fn decode_identity_payload_with_seq(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088IdentityFrame, Bmi088DecodeError> {
    let mut offset = 0;
    let mut identity_format_version = None;
    let mut device_name = None;
    let mut board_name = None;
    let mut firmware_version = None;
    let mut protocol_name = None;
    let mut protocol_version = None;
    let mut transport_name = None;
    let mut sample_rate_hz = None;
    let mut schema_field_count = None;
    let mut sample_payload_len = None;
    let mut protocol_version_byte = None;
    let mut feature_flags = None;
    let mut baud_rate = None;
    let mut protocol_minor_version = None;

    while offset < payload.len() {
        if payload.len() < offset + 2 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "identity TLV header too short".to_string(),
            ));
        }

        let tag = payload[offset];
        let value_len = payload[offset + 1] as usize;
        offset += 2;

        let value = payload.get(offset..offset + value_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("identity TLV value truncated".to_string())
        })?;
        offset += value_len;

        match tag {
            0x00 => {
                identity_format_version = Some(decode_tlv_u8(value, "identity format version")?)
            }
            0x01 => device_name = Some(decode_tlv_string(value, "device name")?),
            0x02 => board_name = Some(decode_tlv_string(value, "board name")?),
            0x03 => firmware_version = Some(decode_tlv_string(value, "firmware version")?),
            0x04 => protocol_name = Some(decode_tlv_string(value, "protocol name")?),
            0x05 => protocol_version = Some(decode_tlv_string(value, "protocol version")?),
            0x06 => transport_name = Some(decode_tlv_string(value, "transport name")?),
            0x07 => sample_rate_hz = Some(decode_tlv_u16(value, "sample rate")?),
            0x08 => schema_field_count = Some(decode_tlv_u8(value, "schema field count")?),
            0x09 => sample_payload_len = Some(decode_tlv_u8(value, "sample payload length")?),
            0x0A => protocol_version_byte = Some(decode_tlv_u8(value, "protocol version byte")?),
            0x0B => feature_flags = Some(decode_tlv_u16(value, "feature flags")?),
            0x0C => baud_rate = Some(decode_tlv_u32(value, "baud rate")?),
            0x0D => protocol_minor_version = Some(decode_tlv_u8(value, "protocol minor version")?),
            _ => {}
        }
    }

    Ok(Bmi088IdentityFrame {
        seq,
        identity_format_version: require_tlv_field(
            identity_format_version,
            "identity format version",
        )?,
        device_name: require_tlv_field(device_name, "device name")?,
        board_name: require_tlv_field(board_name, "board name")?,
        firmware_version: require_tlv_field(firmware_version, "firmware version")?,
        protocol_name: require_tlv_field(protocol_name, "protocol name")?,
        protocol_version: require_tlv_field(protocol_version, "protocol version")?,
        transport_name: require_tlv_field(transport_name, "transport name")?,
        sample_rate_hz: require_tlv_field(sample_rate_hz, "sample rate")?,
        schema_field_count: require_tlv_field(schema_field_count, "schema field count")?,
        sample_payload_len: require_tlv_field(sample_payload_len, "sample payload length")?,
        protocol_version_byte: require_tlv_field(protocol_version_byte, "protocol version byte")?,
        feature_flags: require_tlv_field(feature_flags, "feature flags")?,
        baud_rate: require_tlv_field(baud_rate, "baud rate")?,
        protocol_minor_version: require_tlv_field(
            protocol_minor_version,
            "protocol minor version",
        )?,
    })
}

// ── Schema payload decode ────────────────────────────────────────────────────

pub fn decode_schema_payload(payload: &[u8]) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    decode_schema_payload_with_seq(0, payload)
}

pub fn decode_schema_payload_with_seq(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    eprintln!(
        "[bmi088][schema] decode entry seq={} bytes={} first_byte=0x{:02X} preview={}",
        seq,
        payload.len(),
        payload.first().copied().unwrap_or(0),
        hex_preview(payload, 24),
    );

    if payload.is_empty() {
        return Err(Bmi088DecodeError::MalformedFrame(
            "schema payload too short".to_string(),
        ));
    }

    if payload[0] != BMI088_SCHEMA_VERSION {
        eprintln!(
            "[bmi088][schema] branch=legacy seq={} first_byte=0x{:02X}",
            seq, payload[0],
        );
        return decode_legacy_schema_payload_with_seq(seq, payload);
    }

    eprintln!(
        "[bmi088][schema] branch=framed seq={} version={} rate_hz={} field_count={} sample_len={}",
        seq,
        payload[0],
        payload.get(1).copied().unwrap_or(0),
        payload.get(2).copied().unwrap_or(0),
        payload.get(3).copied().unwrap_or(0),
    );

    if payload.len() < 4 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "schema payload too short".to_string(),
        ));
    }

    let rate_hz = payload[1] as u32;
    let field_count = payload[2] as usize;
    let sample_len = payload[3] as usize;
    let compact_len = 4 + field_count * 5;

    if payload.len() == compact_len {
        let mut offset = 4;
        let mut fields = Vec::with_capacity(field_count);

        for _ in 0..field_count {
            if payload.len() < offset + 5 {
                return Err(Bmi088DecodeError::MalformedFrame(
                    "field descriptor too short".to_string(),
                ));
            }

            let field_id = payload[offset];
            let field_type = payload[offset + 1];
            if field_type != BMI088_FIELD_TYPE_I16 {
                return Err(Bmi088DecodeError::MalformedFrame(
                    "unsupported field type".to_string(),
                ));
            }
            let scale_q = payload[offset + 2] as i8;
            let name_code = payload[offset + 3];
            let unit_code = payload[offset + 4];
            offset += 5;

            let name = compact_field_name(name_code).ok_or_else(|| {
                Bmi088DecodeError::MalformedFrame("unknown compact field name".to_string())
            })?;
            let unit = compact_unit_name(unit_code).ok_or_else(|| {
                Bmi088DecodeError::MalformedFrame("unknown compact field unit".to_string())
            })?;

            fields.push(Bmi088FieldDescriptor {
                field_id,
                field_type,
                name,
                unit,
                scale_q,
            });
        }

        if sample_len != fields.len() * 2 {
            return Err(Bmi088DecodeError::SchemaMismatch(
                "sample length does not match i16 field count".to_string(),
            ));
        }

        return Ok(Bmi088SchemaFrame {
            seq,
            schema_version: BMI088_SCHEMA_VERSION,
            rate_hz,
            sample_len,
            fields,
        });
    }

    let mut offset = 4;
    let mut fields = Vec::with_capacity(field_count);

    for _ in 0..field_count {
        if payload.len() < offset + 5 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "field descriptor too short".to_string(),
            ));
        }

        let _field_id = payload[offset];
        let field_type = payload[offset + 1];
        if field_type != BMI088_FIELD_TYPE_I16 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "unsupported field type".to_string(),
            ));
        }
        let scale_q = payload[offset + 2] as i8;
        let name_len = payload[offset + 3] as usize;
        let unit_len = payload[offset + 4] as usize;
        offset += 5;

        let name_bytes = payload.get(offset..offset + name_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing field name bytes".to_string())
        })?;
        let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid field name utf8".to_string())
        })?;
        offset += name_len;

        let unit_bytes = payload
            .get(offset..offset + unit_len)
            .ok_or_else(|| Bmi088DecodeError::MalformedFrame("missing unit bytes".to_string()))?;
        let unit = String::from_utf8(unit_bytes.to_vec())
            .map_err(|_| Bmi088DecodeError::MalformedFrame("invalid unit utf8".to_string()))?;
        offset += unit_len;

        fields.push(Bmi088FieldDescriptor {
            field_id: fields.len() as u8,
            field_type,
            name,
            unit,
            scale_q,
        });
    }

    if offset != payload.len() {
        return Err(Bmi088DecodeError::MalformedFrame(
            "schema payload has trailing bytes".to_string(),
        ));
    }

    if sample_len != fields.len() * 2 {
        return Err(Bmi088DecodeError::SchemaMismatch(
            "sample length does not match i16 field count".to_string(),
        ));
    }

    Ok(Bmi088SchemaFrame {
        seq,
        schema_version: BMI088_SCHEMA_VERSION,
        rate_hz,
        sample_len,
        fields,
    })
}

fn decode_legacy_schema_payload_with_seq(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    match decode_legacy_schema_payload_with_short_descriptors(seq, payload) {
        Ok(schema) => return Ok(schema),
        Err(error) => {
            eprintln!(
                "[bmi088][schema] legacy variant=short-descriptor failed seq={} error={}",
                seq,
                decode_error_label(&error),
            );
        }
    }

    decode_legacy_schema_payload_with_mixed_descriptors(seq, payload)
}

fn decode_legacy_schema_payload_with_short_descriptors(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    let mut offset = 0;
    let mut fields = Vec::new();

    while offset < payload.len() {
        if payload.len() < offset + 3 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "legacy schema descriptor too short".to_string(),
            ));
        }

        let scale_q = payload[offset] as i8;
        let name_len = payload[offset + 1] as usize;
        let unit_len = payload[offset + 2] as usize;
        offset += 3;

        let name_bytes = payload.get(offset..offset + name_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy field name bytes".to_string())
        })?;
        let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid legacy field name utf8".to_string())
        })?;
        offset += name_len;

        let unit_bytes = payload.get(offset..offset + unit_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy unit bytes".to_string())
        })?;
        let unit = String::from_utf8(unit_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid legacy unit utf8".to_string())
        })?;
        offset += unit_len;

        eprintln!(
            "[bmi088][schema] legacy field index={} scale_q={} name={} unit={} next_offset={}",
            fields.len(),
            scale_q,
            name,
            unit,
            offset,
        );

        fields.push(Bmi088FieldDescriptor {
            field_id: fields.len() as u8,
            field_type: BMI088_FIELD_TYPE_I16,
            name,
            unit,
            scale_q,
        });
    }

    if fields.is_empty() {
        return Err(Bmi088DecodeError::MalformedFrame(
            "legacy schema contains no fields".to_string(),
        ));
    }

    eprintln!(
        "[bmi088][schema] legacy summary seq={} field_count={} sample_len={}",
        seq,
        fields.len(),
        fields.len() * 2,
    );

    Ok(Bmi088SchemaFrame {
        seq,
        schema_version: BMI088_SCHEMA_VERSION,
        rate_hz: default_schema().rate_hz,
        sample_len: fields.len() * 2,
        fields,
    })
}

fn decode_legacy_schema_payload_with_mixed_descriptors(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    let mut offset = 0;
    let mut fields = Vec::new();

    let (scale_q, name, unit, next_offset) = decode_legacy_short_descriptor(payload, offset)?;
    eprintln!(
        "[bmi088][schema] legacy-mixed first field index=0 scale_q={} name={} unit={} next_offset={}",
        scale_q, name, unit, next_offset,
    );
    offset = next_offset;
    fields.push(Bmi088FieldDescriptor {
        field_id: 0,
        field_type: BMI088_FIELD_TYPE_I16,
        name,
        unit,
        scale_q,
    });

    while offset < payload.len() {
        if payload.len() < offset + 5 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "legacy mixed descriptor too short".to_string(),
            ));
        }

        let field_id = payload[offset];
        let field_type = payload[offset + 1];
        if field_type != BMI088_FIELD_TYPE_I16 {
            return Err(Bmi088DecodeError::MalformedFrame(
                "unsupported legacy mixed field type".to_string(),
            ));
        }
        let scale_q = payload[offset + 2] as i8;
        let name_len = payload[offset + 3] as usize;
        let unit_len = payload[offset + 4] as usize;
        offset += 5;

        let name_bytes = payload.get(offset..offset + name_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy mixed field name bytes".to_string())
        })?;
        let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid legacy mixed field name utf8".to_string())
        })?;
        offset += name_len;

        let unit_bytes = payload.get(offset..offset + unit_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy mixed unit bytes".to_string())
        })?;
        let unit = String::from_utf8(unit_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid legacy mixed unit utf8".to_string())
        })?;
        offset += unit_len;

        eprintln!(
            "[bmi088][schema] legacy-mixed field index={} field_id={} scale_q={} name={} unit={} next_offset={}",
            fields.len(),
            field_id,
            scale_q,
            name,
            unit,
            offset,
        );

        fields.push(Bmi088FieldDescriptor {
            field_id,
            field_type,
            name,
            unit,
            scale_q,
        });
    }

    eprintln!(
        "[bmi088][schema] legacy variant=mixed summary seq={} field_count={} sample_len={}",
        seq,
        fields.len(),
        fields.len() * 2,
    );

    Ok(Bmi088SchemaFrame {
        seq,
        schema_version: BMI088_SCHEMA_VERSION,
        rate_hz: default_schema().rate_hz,
        sample_len: fields.len() * 2,
        fields,
    })
}

fn decode_legacy_short_descriptor(
    payload: &[u8],
    offset: usize,
) -> Result<(i8, String, String, usize), Bmi088DecodeError> {
    if payload.len() < offset + 3 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "legacy schema descriptor too short".to_string(),
        ));
    }

    let scale_q = payload[offset] as i8;
    let name_len = payload[offset + 1] as usize;
    let unit_len = payload[offset + 2] as usize;
    let mut next_offset = offset + 3;

    let name_bytes = payload
        .get(next_offset..next_offset + name_len)
        .ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy field name bytes".to_string())
        })?;
    let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
        Bmi088DecodeError::MalformedFrame("invalid legacy field name utf8".to_string())
    })?;
    next_offset += name_len;

    let unit_bytes = payload
        .get(next_offset..next_offset + unit_len)
        .ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing legacy unit bytes".to_string())
        })?;
    let unit = String::from_utf8(unit_bytes.to_vec())
        .map_err(|_| Bmi088DecodeError::MalformedFrame("invalid legacy unit utf8".to_string()))?;
    next_offset += unit_len;

    Ok((scale_q, name, unit, next_offset))
}

// ── Sample payload decode ────────────────────────────────────────────────────

pub fn decode_sample_payload(payload: &[u8]) -> Result<Bmi088SampleFrame, Bmi088DecodeError> {
    let schema = default_schema();
    decode_sample_payload_with_schema_and_seq(payload, &schema, 0)
}

pub fn decode_sample_payload_with_schema(
    payload: &[u8],
    schema: &Bmi088SchemaFrame,
) -> Result<Bmi088SampleFrame, Bmi088DecodeError> {
    decode_sample_payload_with_schema_and_seq(payload, schema, 0)
}

pub fn decode_sample_payload_with_schema_and_seq(
    payload: &[u8],
    schema: &Bmi088SchemaFrame,
    seq: u8,
) -> Result<Bmi088SampleFrame, Bmi088DecodeError> {
    if payload.len() != schema.sample_len {
        return Err(Bmi088DecodeError::SchemaMismatch(
            "sample length does not match expected schema".to_string(),
        ));
    }

    if payload.len() % 2 != 0 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "sample payload must contain i16 values".to_string(),
        ));
    }

    let raw_values = payload
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();

    let mut sample = Bmi088SampleFrame::from_raw_values(&schema, &raw_values)?;
    sample.seq = seq;
    Ok(sample)
}

pub fn decode_sample_raw_values(payload: &[u8]) -> Result<Vec<i16>, Bmi088DecodeError> {
    if payload.len() % 2 != 0 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "sample payload must contain i16 values".to_string(),
        ));
    }

    Ok(payload
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

// ── Frame envelope helpers ───────────────────────────────────────────────────

pub fn frame_len_from_payload_len(payload_len: u8) -> usize {
    BMI088_HEADER_LEN + payload_len as usize + BMI088_CRC_LEN
}

pub fn frame_len(frame: &[u8]) -> Result<usize, Bmi088DecodeError> {
    if frame.len() < BMI088_MIN_FRAME_LEN {
        return Err(Bmi088DecodeError::MalformedFrame(
            "frame too short".to_string(),
        ));
    }

    Ok(frame_len_from_payload_len(frame[6]))
}

pub fn decode_frame_envelope(frame: &[u8]) -> Result<(u8, u8, u8, &[u8]), Bmi088DecodeError> {
    if frame.len() < BMI088_MIN_FRAME_LEN {
        return Err(Bmi088DecodeError::MalformedFrame(
            "frame too short".to_string(),
        ));
    }
    if frame[0..2] != BMI088_SOF {
        return Err(Bmi088DecodeError::InvalidSof);
    }
    if frame[2] != BMI088_VERSION {
        return Err(Bmi088DecodeError::InvalidVersion);
    }

    let expected_len = frame_len(frame)?;
    if frame.len() != expected_len {
        return Err(Bmi088DecodeError::MalformedFrame(
            "length mismatch".to_string(),
        ));
    }

    let crc = u16::from_le_bytes([frame[expected_len - 2], frame[expected_len - 1]]);
    let calculated_crc = crc16_ccitt(&frame[..expected_len - 2]);
    if crc != calculated_crc {
        return Err(Bmi088DecodeError::InvalidCrc);
    }

    Ok((
        frame[3],
        frame[4],
        frame[5],
        &frame[BMI088_HEADER_LEN..expected_len - 2],
    ))
}

pub fn find_sof(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == BMI088_SOF)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn require_tlv_field<T>(value: Option<T>, field_name: &str) -> Result<T, Bmi088DecodeError> {
    value.ok_or_else(|| {
        Bmi088DecodeError::MalformedFrame(format!("missing identity field: {field_name}"))
    })
}

fn decode_tlv_string(value: &[u8], field_name: &str) -> Result<String, Bmi088DecodeError> {
    String::from_utf8(value.to_vec())
        .map_err(|_| Bmi088DecodeError::MalformedFrame(format!("invalid {field_name} utf8")))
}

fn decode_tlv_u8(value: &[u8], field_name: &str) -> Result<u8, Bmi088DecodeError> {
    if value.len() != 1 {
        return Err(Bmi088DecodeError::MalformedFrame(format!(
            "invalid {field_name} size"
        )));
    }
    Ok(value[0])
}

fn decode_tlv_u16(value: &[u8], field_name: &str) -> Result<u16, Bmi088DecodeError> {
    if value.len() != 2 {
        return Err(Bmi088DecodeError::MalformedFrame(format!(
            "invalid {field_name} size"
        )));
    }
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn decode_tlv_u32(value: &[u8], field_name: &str) -> Result<u32, Bmi088DecodeError> {
    if value.len() != 4 {
        return Err(Bmi088DecodeError::MalformedFrame(format!(
            "invalid {field_name} size"
        )));
    }
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn decode_error_label(error: &Bmi088DecodeError) -> String {
    match error {
        Bmi088DecodeError::InvalidSof => "InvalidSof".to_string(),
        Bmi088DecodeError::InvalidVersion => "InvalidVersion".to_string(),
        Bmi088DecodeError::InvalidCrc => "InvalidCrc".to_string(),
        Bmi088DecodeError::MalformedFrame(reason) => format!("MalformedFrame({reason})"),
        Bmi088DecodeError::SchemaMismatch(reason) => format!("SchemaMismatch({reason})"),
    }
}

fn hex_preview(bytes: &[u8], max_len: usize) -> String {
    let preview = bytes
        .iter()
        .take(max_len)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    if bytes.len() > max_len {
        format!("{preview} ...")
    } else {
        preview
    }
}

fn text_preview(text: &str, max_len: usize) -> String {
    let preview = text.chars().take(max_len).collect::<String>();
    if text.chars().count() > max_len {
        format!("{preview}...")
    } else {
        preview
    }
}
