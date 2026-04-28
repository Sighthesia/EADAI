use crate::message::LinePayload;
use crate::serial::{FramedLine, LineFramer};
use serde::Serialize;

pub const BMI088_SOF: [u8; 2] = [0xA5, 0x5A];
pub const BMI088_VERSION: u8 = 0x01;
pub const BMI088_FRAME_TYPE_REQUEST: u8 = 0x01;
pub const BMI088_FRAME_TYPE_RESPONSE: u8 = 0x02;
pub const BMI088_FRAME_TYPE_EVENT: u8 = 0x03;

pub const BMI088_CMD_ACK: u8 = 0x10;
pub const BMI088_CMD_START: u8 = 0x11;
pub const BMI088_CMD_STOP: u8 = 0x12;
pub const BMI088_CMD_REQ_SCHEMA: u8 = 0x13;
pub const BMI088_CMD_REQ_IDENTITY: u8 = 0x14;
pub const BMI088_CMD_REQ_TUNING: u8 = 0x26;
pub const BMI088_CMD_SET_TUNING: u8 = 0x27;
pub const BMI088_CMD_SHELL_EXEC: u8 = 0x28;
pub const BMI088_CMD_SCHEMA: u8 = 0x80;
pub const BMI088_CMD_SAMPLE: u8 = 0x81;
pub const BMI088_CMD_IDENTITY: u8 = 0x82;
pub const BMI088_CMD_SHELL_OUTPUT: u8 = 0x83;
pub const BMI088_SCHEMA_VERSION: u8 = 0x01;
pub const BMI088_FIELD_TYPE_I16: u8 = 0x01;

pub const BMI088_SAMPLE_FIELD_NAMES: [&str; 30] = [
    "acc_x",
    "acc_y",
    "acc_z",
    "gyro_x",
    "gyro_y",
    "gyro_z",
    "roll",
    "pitch",
    "yaw",
    // FIXME: The pasted contract lists 30 fields but only names 28 of them.
    "reserved_0",
    "reserved_1",
    "motor_left_rear_wheel",
    "motor_left_front_wheel",
    "motor_right_front_wheel",
    "motor_right_rear_wheel",
    "roll_correction_output",
    "pitch_correction_output",
    "yaw_correction_output",
    "throttle_correction_output",
    "roll_proportional_gain_x100",
    "roll_integral_gain_x100",
    "roll_derivative_gain_x100",
    "pitch_proportional_gain_x100",
    "pitch_integral_gain_x100",
    "pitch_derivative_gain_x100",
    "yaw_proportional_gain_x100",
    "yaw_integral_gain_x100",
    "yaw_derivative_gain_x100",
    "output_limit",
    "bench_test_throttle",
];
pub const BMI088_SAMPLE_UNITS: [&str; 30] = [
    "raw", "raw", "raw", "raw", "raw", "raw", "deg", "deg", "deg",
    "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw",
    "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw", "raw",
    "raw",
];
pub const BMI088_SAMPLE_SCALE_Q: [i8; 30] = [
    0, 0, 0, 0, 0, 0, -2, -2, -2,
    0, 0,
    0, 0, 0, 0,
    0, 0, 0, 0,
    -2, -2, -2,
    -2, -2, -2,
    -2, -2, -2,
    0, 0,
];

pub const BMI088_HEADER_LEN: usize = 7;
pub const BMI088_CRC_LEN: usize = 2;
pub const BMI088_MIN_FRAME_LEN: usize = BMI088_HEADER_LEN + BMI088_CRC_LEN;

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

#[derive(Clone, Debug)]
pub struct Bmi088SessionState {
    phase: Bmi088SessionPhase,
    identity: Option<Bmi088IdentityFrame>,
    schema: Option<Bmi088SchemaFrame>,
    boot_commands_sent: bool,
}

#[derive(Clone, Debug)]
pub struct Bmi088StreamDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
    text_framer: LineFramer,
    schema: Option<Bmi088SchemaFrame>,
    identity: Option<Bmi088IdentityFrame>,
}

impl Default for Bmi088SchemaFrame {
    fn default() -> Self {
        Self::bmi088_telemetry()
    }
}

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

impl Bmi088SessionState {
    pub fn new() -> Self {
        Self {
            phase: Bmi088SessionPhase::AwaitingSchema,
            identity: None,
            schema: None,
            boot_commands_sent: false,
        }
    }

    pub fn boot_commands(&mut self) -> Vec<Bmi088HostCommand> {
        if self.boot_commands_sent {
            return Vec::new();
        }

        self.boot_commands_sent = true;
        self.phase = Bmi088SessionPhase::AwaitingSchema;
        vec![Bmi088HostCommand::ReqSchema]
    }

    pub fn schema_retry_commands(&self) -> Vec<Bmi088HostCommand> {
        if self.phase == Bmi088SessionPhase::AwaitingSchema {
            vec![Bmi088HostCommand::ReqSchema]
        } else {
            Vec::new()
        }
    }

    pub fn phase(&self) -> Bmi088SessionPhase {
        self.phase
    }

    pub fn schema(&self) -> Option<&Bmi088SchemaFrame> {
        self.schema.as_ref()
    }

    pub fn identity(&self) -> Option<&Bmi088IdentityFrame> {
        self.identity.as_ref()
    }

    pub fn on_frame(&mut self, frame: &Bmi088Frame) -> Vec<Bmi088HostCommand> {
        let previous_phase = self.phase;
        match frame {
            Bmi088Frame::Identity(identity) => {
                self.identity = Some(identity.clone());
                eprintln!(
                    "[bmi088][session] rx IDENTITY seq={} phase={:?} device={} protocol={} schema_fields={} sample_len={}",
                    identity.seq,
                    previous_phase,
                    identity.device_name,
                    identity.protocol_version,
                    identity.schema_field_count,
                    identity.sample_payload_len,
                );
                Vec::new()
            }
            Bmi088Frame::ShellOutput(_) => Vec::new(),
            Bmi088Frame::Schema(schema) => {
                self.schema = Some(schema.clone());
                if matches!(self.phase, Bmi088SessionPhase::Streaming | Bmi088SessionPhase::Stopped) {
                    eprintln!(
                        "[bmi088][session] rx SCHEMA seq={} phase={:?} field_count={} sample_len={} ignored_rehandshake=true",
                        schema.seq,
                        previous_phase,
                        schema.fields.len(),
                        schema.sample_len,
                    );
                    return Vec::new();
                }
                self.phase = Bmi088SessionPhase::AwaitingAck;
                eprintln!(
                    "[bmi088][session] rx SCHEMA seq={} phase={:?}->{:?} field_count={} sample_len={} next=ACK,START",
                    schema.seq,
                    previous_phase,
                    self.phase,
                    schema.fields.len(),
                    schema.sample_len,
                );
                vec![Bmi088HostCommand::Ack, Bmi088HostCommand::Start]
            }
            Bmi088Frame::Sample(_) => {
                self.phase = Bmi088SessionPhase::Streaming;
                eprintln!(
                    "[bmi088][session] rx SAMPLE phase={:?}->{:?}",
                    previous_phase,
                    self.phase,
                );
                Vec::new()
            }
        }
    }

    pub fn on_host_command(&mut self, command: Bmi088HostCommand) {
        let previous_phase = self.phase;
        self.phase = match command {
            Bmi088HostCommand::Ack => Bmi088SessionPhase::AwaitingStart,
            Bmi088HostCommand::Start => Bmi088SessionPhase::Streaming,
            Bmi088HostCommand::Stop => Bmi088SessionPhase::Stopped,
            Bmi088HostCommand::ReqSchema => Bmi088SessionPhase::AwaitingSchema,
            Bmi088HostCommand::ReqIdentity
            | Bmi088HostCommand::ReqTuning
            | Bmi088HostCommand::SetTuning
            | Bmi088HostCommand::ShellExec => self.phase,
        };
        eprintln!(
            "[bmi088][session] tx {} phase={:?}->{:?}",
            host_command_label(&command),
            previous_phase,
            self.phase,
        );
    }
}

impl Default for Bmi088SessionState {
    fn default() -> Self {
        Self::new()
    }
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
                        payload_len,
                        frame_len,
                        self.max_buffer_bytes,
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
                            identity.seq,
                            payload_len,
                            frame_len,
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
                        if let Err(error) = decode_binary_frame_with_schema(&frame, self.schema.as_ref()) {
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
                        if let Err(error) = decode_binary_frame_with_schema(&frame, self.schema.as_ref()) {
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
}

impl Bmi088StreamDecoder {
    fn should_cache_schema(&self, schema: &Bmi088SchemaFrame) -> bool {
        if let Some(identity) = &self.identity {
            let matches_identity_shape = schema.sample_len == usize::from(identity.sample_payload_len)
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
        (
            BMI088_FRAME_TYPE_EVENT | BMI088_FRAME_TYPE_RESPONSE,
            BMI088_CMD_IDENTITY,
        ) => Ok(Bmi088Frame::Identity(
            decode_identity_payload_with_seq(seq, payload)?,
        )),
        (BMI088_FRAME_TYPE_EVENT | BMI088_FRAME_TYPE_RESPONSE, BMI088_CMD_SCHEMA) => {
            Ok(Bmi088Frame::Schema(decode_schema_payload_with_seq(seq, payload)?))
        }
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SHELL_OUTPUT) => Ok(Bmi088Frame::ShellOutput(
            LinePayload {
                text: String::from_utf8_lossy(payload).into_owned(),
                raw: payload.to_vec(),
            },
        )),
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
                    Ok(Bmi088Frame::Sample(decode_sample_payload_with_schema_and_seq(
                        payload,
                        &fallback_schema,
                        seq,
                    )?))
                }
                Err(error) => Err(error),
            }
        }
        _ => Err(Bmi088DecodeError::MalformedFrame(
            "unsupported command".to_string(),
        )),
    }
}

pub fn decode_identity_payload(
    payload: &[u8],
) -> Result<Bmi088IdentityFrame, Bmi088DecodeError> {
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
            0x00 => identity_format_version = Some(decode_tlv_u8(value, "identity format version")?),
            0x01 => device_name = Some(decode_tlv_string(value, "device name")?),
            0x02 => board_name = Some(decode_tlv_string(value, "board name")?),
            0x03 => firmware_version = Some(decode_tlv_string(value, "firmware version")?),
            0x04 => protocol_name = Some(decode_tlv_string(value, "protocol name")?),
            0x05 => protocol_version = Some(decode_tlv_string(value, "protocol version")?),
            0x06 => transport_name = Some(decode_tlv_string(value, "transport name")?),
            0x07 => sample_rate_hz = Some(decode_tlv_u16(value, "sample rate")?),
            0x08 => schema_field_count = Some(decode_tlv_u8(value, "schema field count")?),
            0x09 => sample_payload_len = Some(decode_tlv_u8(value, "sample payload length")?),
            0x0A => {
                protocol_version_byte = Some(decode_tlv_u8(value, "protocol version byte")?)
            }
            0x0B => feature_flags = Some(decode_tlv_u16(value, "feature flags")?),
            0x0C => baud_rate = Some(decode_tlv_u32(value, "baud rate")?),
            0x0D => {
                protocol_minor_version = Some(decode_tlv_u8(value, "protocol minor version")?)
            }
            _ => {}
        }
    }

    Ok(Bmi088IdentityFrame {
        seq,
        identity_format_version: require_tlv_field(identity_format_version, "identity format version")?,
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
        protocol_minor_version: require_tlv_field(protocol_minor_version, "protocol minor version")?,
    })
}

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
            seq,
            payload[0],
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
        scale_q,
        name,
        unit,
        next_offset,
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
            Bmi088DecodeError::MalformedFrame(
                "missing legacy mixed field name bytes".to_string(),
            )
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

    let name_bytes = payload.get(next_offset..next_offset + name_len).ok_or_else(|| {
        Bmi088DecodeError::MalformedFrame("missing legacy field name bytes".to_string())
    })?;
    let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
        Bmi088DecodeError::MalformedFrame("invalid legacy field name utf8".to_string())
    })?;
    next_offset += name_len;

    let unit_bytes = payload.get(next_offset..next_offset + unit_len).ok_or_else(|| {
        Bmi088DecodeError::MalformedFrame("missing legacy unit bytes".to_string())
    })?;
    let unit = String::from_utf8(unit_bytes.to_vec()).map_err(|_| {
        Bmi088DecodeError::MalformedFrame("invalid legacy unit utf8".to_string())
    })?;
    next_offset += unit_len;

    Ok((scale_q, name, unit, next_offset))
}

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

pub fn decode_frame_envelope(
    frame: &[u8],
) -> Result<(u8, u8, u8, &[u8]), Bmi088DecodeError> {
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

    Ok((frame[3], frame[4], frame[5], &frame[BMI088_HEADER_LEN..expected_len - 2]))
}

pub fn find_sof(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == BMI088_SOF)
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

fn encode_frame(frame_type: u8, command: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
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

fn require_tlv_field<T>(value: Option<T>, field_name: &str) -> Result<T, Bmi088DecodeError> {
    value.ok_or_else(|| {
        Bmi088DecodeError::MalformedFrame(format!("missing identity field: {field_name}"))
    })
}

fn decode_tlv_string(value: &[u8], field_name: &str) -> Result<String, Bmi088DecodeError> {
    String::from_utf8(value.to_vec()).map_err(|_| {
        Bmi088DecodeError::MalformedFrame(format!("invalid {field_name} utf8"))
    })
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

fn compact_field_code(name: &str) -> Option<u8> {
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

fn compact_field_name(code: u8) -> Option<String> {
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

fn compact_unit_code(unit: &str) -> Option<u8> {
    match unit {
        "raw" => Some(0x00),
        "deg" => Some(0x01),
        _ => None,
    }
}

fn compact_unit_name(code: u8) -> Option<String> {
    Some(match code {
        0x00 => "raw",
        0x01 => "deg",
        _ => return None,
    }
    .to_string())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bmi088DecodeError {
    InvalidSof,
    InvalidVersion,
    InvalidCrc,
    SchemaMismatch(String),
    MalformedFrame(String),
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
