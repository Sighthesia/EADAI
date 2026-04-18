use crate::serial::{FramedLine, LineFramer};
use serde::Serialize;

pub const BMI088_SOF: [u8; 2] = [0xA5, 0x5A];
pub const BMI088_VERSION: u8 = 0x01;
pub const BMI088_FRAME_TYPE_REQUEST: u8 = 0x01;
pub const BMI088_FRAME_TYPE_EVENT: u8 = 0x02;

pub const BMI088_CMD_ACK: u8 = 0x10;
pub const BMI088_CMD_START: u8 = 0x11;
pub const BMI088_CMD_STOP: u8 = 0x12;
pub const BMI088_CMD_REQ_SCHEMA: u8 = 0x13;
pub const BMI088_CMD_SCHEMA: u8 = 0x80;
pub const BMI088_CMD_SAMPLE: u8 = 0x81;

pub const BMI088_SAMPLE_FIELD_NAMES: [&str; 9] = [
    "acc_x", "acc_y", "acc_z", "gyro_x", "gyro_y", "gyro_z", "roll", "pitch", "yaw",
];
pub const BMI088_SAMPLE_UNITS: [&str; 9] = [
    "g", "g", "g", "deg/s", "deg/s", "deg/s", "deg", "deg", "deg",
];
pub const BMI088_SAMPLE_SCALE_Q: [i8; 9] = [0, 0, 0, 0, 0, 0, -2, -2, -2];

const BMI088_HEADER_LEN: usize = 7;
const BMI088_CRC_LEN: usize = 2;
const BMI088_MIN_FRAME_LEN: usize = BMI088_HEADER_LEN + BMI088_CRC_LEN;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088FieldDescriptor {
    pub name: String,
    pub unit: String,
    pub scale_q: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bmi088SchemaFrame {
    pub rate_hz: u32,
    pub sample_len: usize,
    pub fields: Vec<Bmi088FieldDescriptor>,
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
    pub fields: Vec<Bmi088SampleField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bmi088HostCommand {
    Ack,
    Start,
    Stop,
    ReqSchema,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bmi088Frame {
    Schema(Bmi088SchemaFrame),
    Sample(Bmi088SampleFrame),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TelemetryPacket {
    Text(FramedLine),
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
    schema: Option<Bmi088SchemaFrame>,
}

#[derive(Clone, Debug)]
pub struct Bmi088StreamDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
    text_framer: LineFramer,
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
            .map(|((name, unit), scale_q)| Bmi088FieldDescriptor {
                name: (*name).to_string(),
                unit: (*unit).to_string(),
                scale_q: *scale_q,
            })
            .collect();

        Self {
            rate_hz: 100,
            sample_len: BMI088_SAMPLE_FIELD_NAMES.len() * 2,
            fields,
        }
    }

    pub fn encode_payload(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.rate_hz.to_le_bytes());
        payload.extend_from_slice(&(self.sample_len as u16).to_le_bytes());
        payload.push(self.fields.len() as u8);

        for field in &self.fields {
            payload.push(field.name.len() as u8);
            payload.extend_from_slice(field.name.as_bytes());
            payload.push(field.scale_q as u8);
            payload.push(field.unit.len() as u8);
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

        Ok(Self { fields })
    }

    pub fn fields(&self) -> &[Bmi088SampleField] {
        &self.fields
    }
}

impl Bmi088SessionState {
    pub fn new() -> Self {
        Self {
            phase: Bmi088SessionPhase::AwaitingSchema,
            schema: None,
        }
    }

    pub fn boot_commands(&mut self) -> Vec<Vec<u8>> {
        self.phase = Bmi088SessionPhase::AwaitingSchema;
        vec![encode_host_command(Bmi088HostCommand::ReqSchema)]
    }

    pub fn phase(&self) -> Bmi088SessionPhase {
        self.phase
    }

    pub fn schema(&self) -> Option<&Bmi088SchemaFrame> {
        self.schema.as_ref()
    }

    pub fn on_frame(&mut self, frame: &Bmi088Frame) -> Vec<Vec<u8>> {
        match frame {
            Bmi088Frame::Schema(schema) => {
                self.schema = Some(schema.clone());
                self.phase = Bmi088SessionPhase::AwaitingAck;
                vec![
                    encode_host_command(Bmi088HostCommand::Ack),
                    encode_host_command(Bmi088HostCommand::Start),
                ]
            }
            Bmi088Frame::Sample(_) => {
                self.phase = Bmi088SessionPhase::Streaming;
                Vec::new()
            }
        }
    }

    pub fn on_host_command(&mut self, command: Bmi088HostCommand) {
        self.phase = match command {
            Bmi088HostCommand::Ack => Bmi088SessionPhase::AwaitingStart,
            Bmi088HostCommand::Start => Bmi088SessionPhase::Streaming,
            Bmi088HostCommand::Stop => Bmi088SessionPhase::Stopped,
            Bmi088HostCommand::ReqSchema => Bmi088SessionPhase::AwaitingSchema,
        };
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
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<TelemetryPacket> {
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

                let payload_len = u16::from_le_bytes([self.buffer[5], self.buffer[6]]) as usize;
                let frame_len = BMI088_HEADER_LEN + payload_len + BMI088_CRC_LEN;
                if frame_len > self.max_buffer_bytes {
                    self.buffer.drain(..1);
                    continue;
                }

                if self.buffer.len() < frame_len {
                    break;
                }

                let frame = self.buffer[..frame_len].to_vec();
                match decode_binary_frame(&frame) {
                    Ok(Bmi088Frame::Schema(schema)) => {
                        self.buffer.drain(..frame_len);
                        packets.push(TelemetryPacket::Schema(schema));
                    }
                    Ok(Bmi088Frame::Sample(sample)) => {
                        self.buffer.drain(..frame_len);
                        packets.push(TelemetryPacket::Sample(sample));
                    }
                    Err(
                        Bmi088DecodeError::InvalidCrc
                        | Bmi088DecodeError::InvalidVersion
                        | Bmi088DecodeError::InvalidSof,
                    ) => {
                        self.buffer.drain(..1);
                    }
                    Err(Bmi088DecodeError::SchemaMismatch(_))
                    | Err(Bmi088DecodeError::MalformedFrame(_)) => {
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

pub fn encode_host_command(command: Bmi088HostCommand) -> Vec<u8> {
    let command_code = match command {
        Bmi088HostCommand::Ack => BMI088_CMD_ACK,
        Bmi088HostCommand::Start => BMI088_CMD_START,
        Bmi088HostCommand::Stop => BMI088_CMD_STOP,
        Bmi088HostCommand::ReqSchema => BMI088_CMD_REQ_SCHEMA,
    };

    encode_frame(BMI088_FRAME_TYPE_REQUEST, command_code, &[])
}

pub fn host_command_from_text(text: &str) -> Option<Bmi088HostCommand> {
    match text.trim().to_ascii_lowercase().as_str() {
        "ack" => Some(Bmi088HostCommand::Ack),
        "start" => Some(Bmi088HostCommand::Start),
        "stop" => Some(Bmi088HostCommand::Stop),
        "req_schema" => Some(Bmi088HostCommand::ReqSchema),
        _ => None,
    }
}

pub fn encode_schema_frame(schema: &Bmi088SchemaFrame) -> Vec<u8> {
    encode_frame(
        BMI088_FRAME_TYPE_EVENT,
        BMI088_CMD_SCHEMA,
        &schema.encode_payload(),
    )
}

pub fn encode_sample_frame(sample: &Bmi088SampleFrame) -> Vec<u8> {
    let mut payload = Vec::with_capacity(sample.fields.len() * 2);
    for field in &sample.fields {
        payload.extend_from_slice(&field.raw.to_le_bytes());
    }
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SAMPLE, &payload)
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
    ];

    Bmi088SampleFrame::from_raw_values(&default_schema(), &raw_values)
        .expect("default bmi088 sample")
}

pub fn decode_binary_frame(frame: &[u8]) -> Result<Bmi088Frame, Bmi088DecodeError> {
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

    let payload_len = u16::from_le_bytes([frame[5], frame[6]]) as usize;
    let expected_len = BMI088_HEADER_LEN + payload_len + BMI088_CRC_LEN;
    if frame.len() != expected_len {
        return Err(Bmi088DecodeError::MalformedFrame(
            "length mismatch".to_string(),
        ));
    }

    let crc = u16::from_le_bytes([frame[expected_len - 2], frame[expected_len - 1]]);
    let calculated_crc = crc16_ccitt(&frame[2..expected_len - 2]);
    if crc != calculated_crc {
        return Err(Bmi088DecodeError::InvalidCrc);
    }

    let frame_type = frame[3];
    let command = frame[4];
    let payload = &frame[BMI088_HEADER_LEN..expected_len - 2];

    match (frame_type, command) {
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SCHEMA) => {
            Ok(Bmi088Frame::Schema(decode_schema_payload(payload)?))
        }
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SAMPLE) => {
            Ok(Bmi088Frame::Sample(decode_sample_payload(payload)?))
        }
        _ => Err(Bmi088DecodeError::MalformedFrame(
            "unsupported command".to_string(),
        )),
    }
}

pub fn decode_schema_payload(payload: &[u8]) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    if payload.len() < 7 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "schema payload too short".to_string(),
        ));
    }

    let rate_hz = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let sample_len = u16::from_le_bytes([payload[4], payload[5]]) as usize;
    let field_count = payload[6] as usize;
    let mut offset = 7;
    let mut fields = Vec::with_capacity(field_count);

    for _ in 0..field_count {
        let name_len = *payload.get(offset).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing field name length".to_string())
        })? as usize;
        offset += 1;
        let name_bytes = payload.get(offset..offset + name_len).ok_or_else(|| {
            Bmi088DecodeError::MalformedFrame("missing field name bytes".to_string())
        })?;
        let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
            Bmi088DecodeError::MalformedFrame("invalid field name utf8".to_string())
        })?;
        offset += name_len;

        let scale_q = *payload
            .get(offset)
            .ok_or_else(|| Bmi088DecodeError::MalformedFrame("missing scale_q".to_string()))?
            as i8;
        offset += 1;

        let unit_len = *payload
            .get(offset)
            .ok_or_else(|| Bmi088DecodeError::MalformedFrame("missing unit length".to_string()))?
            as usize;
        offset += 1;
        let unit_bytes = payload
            .get(offset..offset + unit_len)
            .ok_or_else(|| Bmi088DecodeError::MalformedFrame("missing unit bytes".to_string()))?;
        let unit = String::from_utf8(unit_bytes.to_vec())
            .map_err(|_| Bmi088DecodeError::MalformedFrame("invalid unit utf8".to_string()))?;
        offset += unit_len;

        fields.push(Bmi088FieldDescriptor {
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

    Ok(Bmi088SchemaFrame {
        rate_hz,
        sample_len,
        fields,
    })
}

pub fn decode_sample_payload(payload: &[u8]) -> Result<Bmi088SampleFrame, Bmi088DecodeError> {
    let schema = default_schema();
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

    Bmi088SampleFrame::from_raw_values(&schema, &raw_values)
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

fn encode_frame(frame_type: u8, command: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(BMI088_HEADER_LEN + payload.len() + BMI088_CRC_LEN);
    frame.extend_from_slice(&BMI088_SOF);
    frame.push(BMI088_VERSION);
    frame.push(frame_type);
    frame.push(command);
    frame.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    frame.extend_from_slice(payload);
    let crc = crc16_ccitt(&frame[2..]);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame
}

fn find_sof(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == BMI088_SOF)
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
