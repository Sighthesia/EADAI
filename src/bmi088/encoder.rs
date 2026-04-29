/// BMI088 frame encoding, CRC, host command encoding, and default data generators.

use super::constants::*;
use super::models::{
    Bmi088DecodeError, Bmi088FieldDescriptor, Bmi088HostCommand, Bmi088IdentityFrame,
    Bmi088SampleField, Bmi088SampleFrame, Bmi088SchemaFrame,
};
use crate::message::LinePayload;

// ── Public encode API ────────────────────────────────────────────────────────

pub fn encode_host_command(command: Bmi088HostCommand) -> Vec<u8> {
    encode_host_command_with_payload(command, &[])
}

pub fn encode_host_command_with_seq(command: Bmi088HostCommand, seq: u8) -> Vec<u8> {
    encode_host_command_with_seq_and_payload(command, seq, &[])
}

pub fn encode_host_command_with_payload(command: Bmi088HostCommand, payload: &[u8]) -> Vec<u8> {
    encode_host_command_with_seq_and_payload(command, 0, payload)
}

pub fn encode_host_command_with_seq_and_payload(
    command: Bmi088HostCommand,
    seq: u8,
    payload: &[u8],
) -> Vec<u8> {
    let command_code = match command {
        Bmi088HostCommand::Ack => BMI088_CMD_ACK,
        Bmi088HostCommand::Start => BMI088_CMD_START,
        Bmi088HostCommand::Stop => BMI088_CMD_STOP,
        Bmi088HostCommand::ReqSchema => BMI088_CMD_REQ_SCHEMA,
        Bmi088HostCommand::ReqIdentity => BMI088_CMD_REQ_IDENTITY,
        Bmi088HostCommand::ReqTuning => BMI088_CMD_REQ_TUNING,
        Bmi088HostCommand::SetTuning => BMI088_CMD_SET_TUNING,
        Bmi088HostCommand::ShellExec => BMI088_CMD_SHELL_EXEC,
    };

    encode_frame(BMI088_FRAME_TYPE_REQUEST, command_code, seq, payload)
}

pub fn host_command_label(command: &Bmi088HostCommand) -> &'static str {
    match command {
        Bmi088HostCommand::Ack => "ACK",
        Bmi088HostCommand::Start => "START",
        Bmi088HostCommand::Stop => "STOP",
        Bmi088HostCommand::ReqSchema => "REQ_SCHEMA",
        Bmi088HostCommand::ReqIdentity => "REQ_IDENTITY",
        Bmi088HostCommand::ReqTuning => "REQ_TUNING",
        Bmi088HostCommand::SetTuning => "SET_TUNING",
        Bmi088HostCommand::ShellExec => "SHELL_EXEC",
    }
}

pub fn host_command_from_text(text: &str) -> Option<Bmi088HostCommand> {
    match text.trim().to_ascii_lowercase().as_str() {
        "ack" => Some(Bmi088HostCommand::Ack),
        "start" => Some(Bmi088HostCommand::Start),
        "stop" => Some(Bmi088HostCommand::Stop),
        "req_schema" => Some(Bmi088HostCommand::ReqSchema),
        "req_identity" => Some(Bmi088HostCommand::ReqIdentity),
        "req_tuning" => Some(Bmi088HostCommand::ReqTuning),
        "set_tuning" => Some(Bmi088HostCommand::SetTuning),
        "shell_exec" => Some(Bmi088HostCommand::ShellExec),
        _ => None,
    }
}

pub fn encode_identity_frame(identity: &Bmi088IdentityFrame) -> Vec<u8> {
    encode_identity_frame_with_seq(identity, identity.seq)
}

pub fn encode_identity_frame_with_seq(identity: &Bmi088IdentityFrame, seq: u8) -> Vec<u8> {
    encode_frame(
        BMI088_FRAME_TYPE_EVENT,
        BMI088_CMD_IDENTITY,
        seq,
        &identity.encode_payload(),
    )
}

pub fn encode_schema_frame(schema: &Bmi088SchemaFrame) -> Vec<u8> {
    encode_schema_frame_with_seq(schema, schema.seq)
}

pub fn encode_schema_frame_with_seq(schema: &Bmi088SchemaFrame, seq: u8) -> Vec<u8> {
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SCHEMA, seq, &schema.encode_payload())
}

pub fn encode_sample_frame(sample: &Bmi088SampleFrame) -> Vec<u8> {
    encode_sample_frame_with_seq(sample, sample.seq)
}

pub fn encode_sample_frame_with_seq(sample: &Bmi088SampleFrame, seq: u8) -> Vec<u8> {
    let mut payload = Vec::with_capacity(sample.fields.len() * 2);
    for field in &sample.fields {
        payload.extend_from_slice(&field.raw.to_le_bytes());
    }
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SAMPLE, seq, &payload)
}

pub fn encode_shell_output_frame(output: &LinePayload, seq: u8) -> Vec<u8> {
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SHELL_OUTPUT, seq, &output.raw)
}

pub fn default_schema() -> Bmi088SchemaFrame {
    Bmi088SchemaFrame::bmi088_telemetry()
}

pub fn default_sample(sample_index: u64) -> Bmi088SampleFrame {
    let phase = sample_index as f64 * 0.18;
    let raw_values = [
        ((phase.sin() * 820.0) as i16),
        (((phase * 1.17).cos() * 760.0) as i16),
        (((phase * 0.83).sin() * 1024.0) as i16),
        (((phase * 1.43).sin() * 240.0) as i16),
        (((phase * 1.11).cos() * 220.0) as i16),
        (((phase * 0.71).sin() * 180.0) as i16),
        (((phase * 0.54).sin() * 36.0) as i16),
        ((((phase * 0.49) + 0.4).sin() * 24.0) as i16),
        (normalize_signed_angle_deg((phase * 22.0).sin() * 65.0) * 4.0) as i16,
        (((phase * 0.35).sin() * 180.0) as i16),
        (((phase * 0.41).cos() * 160.0) as i16),
        (((phase * 0.47).sin() * 150.0) as i16),
        (((phase * 0.53).cos() * 140.0) as i16),
        (((phase * 0.59).sin() * 120.0) as i16),
        (((phase * 0.65).cos() * 110.0) as i16),
        (((phase * 0.71).sin() * 100.0) as i16),
        (((phase * 0.77).cos() * 90.0) as i16),
        (((phase * 0.83).sin() * 80.0) as i16),
        (((phase * 0.89).cos() * 70.0) as i16),
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];

    Bmi088SampleFrame::from_raw_values(&default_schema(), &raw_values)
        .expect("default bmi088 sample")
}

pub fn crc16_ccitt(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFF_u16;

    for byte in bytes {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

pub fn scale_raw(raw: i16, scale_q: i8) -> f64 {
    (raw as f64) * 10f64.powi(scale_q as i32)
}

// ── Internal helpers ─────────────────────────────────────────────────────────

pub(crate) fn encode_frame(frame_type: u8, command: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(BMI088_HEADER_LEN + payload.len() + BMI088_CRC_LEN);
    let payload_len = u8::try_from(payload.len()).expect("BMI088 payload length exceeds u8");
    frame.extend_from_slice(&BMI088_SOF);
    frame.push(BMI088_VERSION);
    frame.push(frame_type);
    frame.push(command);
    frame.push(seq);
    frame.push(payload_len);
    frame.extend_from_slice(payload);
    let crc = crc16_ccitt(&frame);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame
}

fn normalize_signed_angle_deg(value: f64) -> f64 {
    let mut normalized = value % 360.0;
    if normalized > 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}

// ── Compact field/unit code tables ───────────────────────────────────────────

pub(crate) fn compact_field_code(name: &str) -> Option<u8> {
    match name {
        "acc_x" => Some(0x00),
        "acc_y" => Some(0x01),
        "acc_z" => Some(0x02),
        "gyro_x" => Some(0x03),
        "gyro_y" => Some(0x04),
        "gyro_z" => Some(0x05),
        "roll" => Some(0x06),
        "pitch" => Some(0x07),
        "yaw" => Some(0x08),
        "reserved_0" => Some(0x09),
        "reserved_1" => Some(0x0A),
        "motor_left_rear_wheel" => Some(0x0B),
        "motor_left_front_wheel" => Some(0x0C),
        "motor_right_front_wheel" => Some(0x0D),
        "motor_right_rear_wheel" => Some(0x0E),
        "roll_correction_output" => Some(0x0F),
        "pitch_correction_output" => Some(0x10),
        "yaw_correction_output" => Some(0x11),
        "throttle_correction_output" => Some(0x12),
        "roll_proportional_gain_x100" => Some(0x13),
        "roll_integral_gain_x100" => Some(0x14),
        "roll_derivative_gain_x100" => Some(0x15),
        "pitch_proportional_gain_x100" => Some(0x16),
        "pitch_integral_gain_x100" => Some(0x17),
        "pitch_derivative_gain_x100" => Some(0x18),
        "yaw_proportional_gain_x100" => Some(0x19),
        "yaw_integral_gain_x100" => Some(0x1A),
        "yaw_derivative_gain_x100" => Some(0x1B),
        "output_limit" => Some(0x1C),
        "bench_test_throttle" => Some(0x1D),
        _ => None,
    }
}

pub(crate) fn compact_field_name(code: u8) -> Option<String> {
    Some(match code {
        0x00 => "acc_x",
        0x01 => "acc_y",
        0x02 => "acc_z",
        0x03 => "gyro_x",
        0x04 => "gyro_y",
        0x05 => "gyro_z",
        0x06 => "roll",
        0x07 => "pitch",
        0x08 => "yaw",
        0x09 => "reserved_0",
        0x0A => "reserved_1",
        0x0B => "motor_left_rear_wheel",
        0x0C => "motor_left_front_wheel",
        0x0D => "motor_right_front_wheel",
        0x0E => "motor_right_rear_wheel",
        0x0F => "roll_correction_output",
        0x10 => "pitch_correction_output",
        0x11 => "yaw_correction_output",
        0x12 => "throttle_correction_output",
        0x13 => "roll_proportional_gain_x100",
        0x14 => "roll_integral_gain_x100",
        0x15 => "roll_derivative_gain_x100",
        0x16 => "pitch_proportional_gain_x100",
        0x17 => "pitch_integral_gain_x100",
        0x18 => "pitch_derivative_gain_x100",
        0x19 => "yaw_proportional_gain_x100",
        0x1A => "yaw_integral_gain_x100",
        0x1B => "yaw_derivative_gain_x100",
        0x1C => "output_limit",
        0x1D => "bench_test_throttle",
        _ => return None,
    }
    .to_string())
}

pub(crate) fn compact_unit_code(unit: &str) -> Option<u8> {
    match unit {
        "raw" => Some(0x00),
        "deg" => Some(0x01),
        _ => None,
    }
}

pub(crate) fn compact_unit_name(code: u8) -> Option<String> {
    Some(match code {
        0x00 => "raw",
        0x01 => "deg",
        _ => return None,
    }
    .to_string())
}

// ── Model impl blocks that depend on encoder internals ───────────────────────

impl Bmi088SchemaFrame {
    pub fn bmi088_telemetry() -> Self {
        let fields = BMI088_SAMPLE_FIELD_NAMES
            .iter()
            .zip(BMI088_SAMPLE_UNITS.iter())
            .zip(BMI088_SAMPLE_SCALE_Q.iter())
            .enumerate()
            .map(|(index, ((name, unit), scale_q))| Bmi088FieldDescriptor {
                field_id: index as u8,
                field_type: BMI088_FIELD_TYPE_I16,
                name: (*name).to_string(),
                unit: (*unit).to_string(),
                scale_q: *scale_q,
            })
            .collect();

        Self {
            seq: 0,
            schema_version: BMI088_SCHEMA_VERSION,
            rate_hz: 100,
            sample_len: BMI088_SAMPLE_FIELD_NAMES.len() * 2,
            fields,
        }
    }

    pub fn encode_payload(&self) -> Vec<u8> {
        if self.fields.iter().all(|field| field.name != "" && compact_field_code(&field.name).is_some()) {
            let mut payload = Vec::with_capacity(4 + self.fields.len() * 5);
            payload.push(self.schema_version);
            payload.push(self.rate_hz.min(u8::MAX as u32) as u8);
            payload.push(self.fields.len() as u8);
            payload.push(self.sample_len.min(u8::MAX as usize) as u8);

            for field in &self.fields {
                payload.push(field.field_id);
                payload.push(field.field_type);
                payload.push(field.scale_q as u8);
                payload.push(compact_field_code(&field.name).unwrap_or(0xFF));
                payload.push(compact_unit_code(&field.unit).unwrap_or(0xFF));
            }

            return payload;
        }

        let mut payload = Vec::new();
        payload.push(self.schema_version);
        payload.push(self.rate_hz.min(u8::MAX as u32) as u8);
        payload.push(self.fields.len() as u8);
        payload.push(self.sample_len.min(u8::MAX as usize) as u8);

        for field in &self.fields {
            payload.push(field.field_id);
            payload.push(field.field_type);
            payload.push(field.scale_q as u8);
            payload.push(field.name.len() as u8);
            payload.push(field.unit.len() as u8);
            payload.extend_from_slice(field.name.as_bytes());
            payload.extend_from_slice(field.unit.as_bytes());
        }

        payload
    }
}

impl Default for Bmi088SchemaFrame {
    fn default() -> Self {
        Self::bmi088_telemetry()
    }
}

impl Bmi088SampleFrame {
    pub fn from_raw_values(
        schema: &Bmi088SchemaFrame,
        raw_values: &[i16],
    ) -> Result<Self, Bmi088DecodeError> {
        if raw_values.len() != schema.fields.len() {
            return Err(Bmi088DecodeError::SchemaMismatch(
                "sample field count does not match schema".to_string(),
            ));
        }

        let fields = raw_values
            .iter()
            .enumerate()
            .map(|(index, raw)| {
                let descriptor = &schema.fields[index];
                Bmi088SampleField {
                    name: descriptor.name.clone(),
                    raw: *raw,
                    value: scale_raw(*raw, descriptor.scale_q),
                    unit: Some(descriptor.unit.clone()),
                    scale_q: descriptor.scale_q,
                    index,
                }
            })
            .collect();

        Ok(Self { seq: 0, fields })
    }

    pub fn fields(&self) -> &[Bmi088SampleField] {
        &self.fields
    }
}

impl Bmi088IdentityFrame {
    pub fn encode_payload(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        encode_tlv_u8(&mut payload, 0x00, self.identity_format_version);
        encode_tlv_string(&mut payload, 0x01, &self.device_name);
        encode_tlv_string(&mut payload, 0x02, &self.board_name);
        encode_tlv_string(&mut payload, 0x03, &self.firmware_version);
        encode_tlv_string(&mut payload, 0x04, &self.protocol_name);
        encode_tlv_string(&mut payload, 0x05, &self.protocol_version);
        encode_tlv_string(&mut payload, 0x06, &self.transport_name);
        encode_tlv_u16(&mut payload, 0x07, self.sample_rate_hz);
        encode_tlv_u8(&mut payload, 0x08, self.schema_field_count);
        encode_tlv_u8(&mut payload, 0x09, self.sample_payload_len);
        encode_tlv_u8(&mut payload, 0x0A, self.protocol_version_byte);
        encode_tlv_u16(&mut payload, 0x0B, self.feature_flags);
        encode_tlv_u32(&mut payload, 0x0C, self.baud_rate);
        encode_tlv_u8(&mut payload, 0x0D, self.protocol_minor_version);
        payload
    }
}

fn encode_tlv_string(payload: &mut Vec<u8>, tag: u8, value: &str) {
    encode_tlv_bytes(payload, tag, value.as_bytes());
}

fn encode_tlv_u8(payload: &mut Vec<u8>, tag: u8, value: u8) {
    encode_tlv_bytes(payload, tag, &[value]);
}

fn encode_tlv_u16(payload: &mut Vec<u8>, tag: u8, value: u16) {
    encode_tlv_bytes(payload, tag, &value.to_le_bytes());
}

fn encode_tlv_u32(payload: &mut Vec<u8>, tag: u8, value: u32) {
    encode_tlv_bytes(payload, tag, &value.to_le_bytes());
}

fn encode_tlv_bytes(payload: &mut Vec<u8>, tag: u8, value: &[u8]) {
    payload.push(tag);
    payload.push(u8::try_from(value.len()).expect("BMI088 identity TLV length exceeds u8"));
    payload.extend_from_slice(value);
}
