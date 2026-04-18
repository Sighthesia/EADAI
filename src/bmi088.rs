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
pub const BMI088_CMD_SCHEMA: u8 = 0x80;
pub const BMI088_CMD_SAMPLE: u8 = 0x81;
pub const BMI088_SCHEMA_VERSION: u8 = 0x01;
pub const BMI088_FIELD_TYPE_I16: u8 = 0x01;

pub const BMI088_SAMPLE_FIELD_NAMES: [&str; 9] = [
    "acc_x", "acc_y", "acc_z", "gyro_x", "gyro_y", "gyro_z", "roll", "pitch", "yaw",
];
pub const BMI088_SAMPLE_UNITS: [&str; 9] = [
    "raw", "raw", "raw", "raw", "raw", "raw", "deg", "deg", "deg",
];
pub const BMI088_SAMPLE_SCALE_Q: [i8; 9] = [0, 0, 0, 0, 0, 0, -2, -2, -2];

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
    schema: Option<Bmi088SchemaFrame>,
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
            schema: None,
        }
    }

    pub fn boot_commands(&mut self) -> Vec<Bmi088HostCommand> {
        self.phase = Bmi088SessionPhase::AwaitingSchema;
        vec![Bmi088HostCommand::ReqSchema]
    }

    pub fn phase(&self) -> Bmi088SessionPhase {
        self.phase
    }

    pub fn schema(&self) -> Option<&Bmi088SchemaFrame> {
        self.schema.as_ref()
    }

    pub fn on_frame(&mut self, frame: &Bmi088Frame) -> Vec<Bmi088HostCommand> {
        match frame {
            Bmi088Frame::Schema(schema) => {
                self.schema = Some(schema.clone());
                self.phase = Bmi088SessionPhase::AwaitingAck;
                vec![Bmi088HostCommand::Ack, Bmi088HostCommand::Start]
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
            schema: None,
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

                let payload_len = self.buffer[6] as usize;
                let frame_len = BMI088_HEADER_LEN + payload_len + BMI088_CRC_LEN;
                if frame_len > self.max_buffer_bytes {
                    self.buffer.drain(..1);
                    continue;
                }

                if self.buffer.len() < frame_len {
                    break;
                }

                let frame = self.buffer[..frame_len].to_vec();
                match decode_binary_frame_with_schema(&frame, self.schema.as_ref()) {
                    Ok(Bmi088Frame::Schema(schema)) => {
                        self.buffer.drain(..frame_len);
                        self.schema = Some(schema.clone());
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

    encode_frame(BMI088_FRAME_TYPE_REQUEST, command_code, 0, &[])
}

pub fn encode_host_command_with_seq(command: Bmi088HostCommand, seq: u8) -> Vec<u8> {
    let command_code = match command {
        Bmi088HostCommand::Ack => BMI088_CMD_ACK,
        Bmi088HostCommand::Start => BMI088_CMD_START,
        Bmi088HostCommand::Stop => BMI088_CMD_STOP,
        Bmi088HostCommand::ReqSchema => BMI088_CMD_REQ_SCHEMA,
    };

    encode_frame(BMI088_FRAME_TYPE_REQUEST, command_code, seq, &[])
}

pub fn host_command_label(command: &Bmi088HostCommand) -> &'static str {
    match command {
        Bmi088HostCommand::Ack => "ACK",
        Bmi088HostCommand::Start => "START",
        Bmi088HostCommand::Stop => "STOP",
        Bmi088HostCommand::ReqSchema => "REQ_SCHEMA",
    }
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
    encode_schema_frame_with_seq(schema, schema.seq)
}

pub fn encode_schema_frame_with_seq(schema: &Bmi088SchemaFrame, seq: u8) -> Vec<u8> {
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SCHEMA, seq, &schema.encode_payload())
}

pub fn encode_sample_frame(sample: &Bmi088SampleFrame) -> Vec<u8> {
    let mut payload = Vec::with_capacity(sample.fields.len() * 2);
    for field in &sample.fields {
        payload.extend_from_slice(&field.raw.to_le_bytes());
    }
    encode_sample_frame_with_seq(sample, sample.seq)
}

pub fn encode_sample_frame_with_seq(sample: &Bmi088SampleFrame, seq: u8) -> Vec<u8> {
    let mut payload = Vec::with_capacity(sample.fields.len() * 2);
    for field in &sample.fields {
        payload.extend_from_slice(&field.raw.to_le_bytes());
    }
    encode_frame(BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SAMPLE, seq, &payload)
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
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SCHEMA) => {
            Ok(Bmi088Frame::Schema(decode_schema_payload_with_seq(seq, payload)?))
        }
        (BMI088_FRAME_TYPE_EVENT, BMI088_CMD_SAMPLE) => Ok(Bmi088Frame::Sample(
            decode_sample_payload_with_schema_and_seq(payload, schema.unwrap_or(&default_schema()), seq)?,
        )),
        _ => Err(Bmi088DecodeError::MalformedFrame(
            "unsupported command".to_string(),
        )),
    }
}

pub fn decode_schema_payload(payload: &[u8]) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    decode_schema_payload_with_seq(0, payload)
}

pub fn decode_schema_payload_with_seq(
    seq: u8,
    payload: &[u8],
) -> Result<Bmi088SchemaFrame, Bmi088DecodeError> {
    if payload.len() < 4 {
        return Err(Bmi088DecodeError::MalformedFrame(
            "schema payload too short".to_string(),
        ));
    }

    if payload[0] != BMI088_SCHEMA_VERSION {
        return Err(Bmi088DecodeError::MalformedFrame(
            "unsupported schema version".to_string(),
        ));
    }

    let rate_hz = payload[1] as u32;
    let field_count = payload[2] as usize;
    let sample_len = payload[3] as usize;
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
