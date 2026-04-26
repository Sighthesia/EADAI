use crate::bmi088::{
    self, Bmi088DecodeError, Bmi088FieldDescriptor, Bmi088Frame, Bmi088HostCommand,
    Bmi088IdentityFrame, Bmi088SchemaFrame, decode_binary_frame_with_schema,
    decode_frame_envelope, decode_sample_raw_values, encode_host_command_with_seq, find_sof,
    frame_len_from_payload_len,
};
use crate::serial::{FramedLine, LineFramer};

pub const SOF: [u8; 2] = bmi088::BMI088_SOF;
pub const VERSION: u8 = bmi088::BMI088_VERSION;
pub const FRAME_TYPE_REQUEST: u8 = bmi088::BMI088_FRAME_TYPE_REQUEST;
pub const FRAME_TYPE_RESPONSE: u8 = bmi088::BMI088_FRAME_TYPE_RESPONSE;
pub const FRAME_TYPE_EVENT: u8 = bmi088::BMI088_FRAME_TYPE_EVENT;

pub const CMD_ACK: u8 = bmi088::BMI088_CMD_ACK;
pub const CMD_START: u8 = bmi088::BMI088_CMD_START;
pub const CMD_STOP: u8 = bmi088::BMI088_CMD_STOP;
pub const CMD_REQ_SCHEMA: u8 = bmi088::BMI088_CMD_REQ_SCHEMA;
pub const CMD_REQ_IDENTITY: u8 = bmi088::BMI088_CMD_REQ_IDENTITY;
pub const CMD_SCHEMA: u8 = bmi088::BMI088_CMD_SCHEMA;
pub const CMD_SAMPLE: u8 = bmi088::BMI088_CMD_SAMPLE;
pub const CMD_IDENTITY: u8 = bmi088::BMI088_CMD_IDENTITY;

const MIN_FRAME_LEN: usize = bmi088::BMI088_MIN_FRAME_LEN;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostCommand {
    Ack,
    Start,
    Stop,
    ReqSchema,
    ReqIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineMode {
    None,
    Lf,
    Crlf,
}

pub type SchemaField = Bmi088FieldDescriptor;
pub type IdentityFrame = Bmi088IdentityFrame;
pub type SchemaFrame = Bmi088SchemaFrame;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleFrame {
    pub seq: u8,
    pub raw_values: Vec<i16>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Identity(IdentityFrame),
    Schema(SchemaFrame),
    Sample(SampleFrame),
    Unknown {
        frame_type: u8,
        command: u8,
        seq: u8,
        payload_len: u8,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Packet {
    Text(FramedLine),
    Frame(Frame),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    InvalidSof,
    InvalidVersion,
    InvalidCrc,
    Malformed(&'static str),
}

#[derive(Clone, Debug)]
pub struct DiagnosticDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
    text_framer: LineFramer,
    invalid_crc_count: usize,
    malformed_frame_count: usize,
    desync_drop_bytes: usize,
    schema: Option<SchemaFrame>,
}

impl DiagnosticDecoder {
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
            text_framer: LineFramer::new(),
            invalid_crc_count: 0,
            malformed_frame_count: 0,
            desync_drop_bytes: 0,
            schema: None,
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<Packet> {
        self.buffer.extend_from_slice(chunk);
        let mut packets = Vec::new();

        loop {
            if let Some(sof_index) = find_sof(&self.buffer) {
                if sof_index > 0 {
                    let prefix = self.buffer.drain(..sof_index).collect::<Vec<_>>();
                    self.desync_drop_bytes += prefix.len();
                    packets.extend(self.text_framer.push(&prefix).into_iter().map(Packet::Text));
                    continue;
                }

                if self.buffer.len() < MIN_FRAME_LEN {
                    break;
                }

                let payload_len = self.buffer[6];
                let frame_len = frame_len_from_payload_len(payload_len);
                if frame_len > self.max_buffer_bytes {
                    self.buffer.drain(..1);
                    self.malformed_frame_count += 1;
                    self.desync_drop_bytes += 1;
                    continue;
                }

                if self.buffer.len() < frame_len {
                    break;
                }

                let frame = self.buffer[..frame_len].to_vec();
                match decode_frame_with_schema(&frame, self.schema.as_ref()) {
                    Ok(Frame::Schema(schema)) => {
                        self.buffer.drain(..frame_len);
                        self.schema = Some(schema.clone());
                        packets.push(Packet::Frame(Frame::Schema(schema)));
                    }
                    Ok(other) => {
                        self.buffer.drain(..frame_len);
                        packets.push(Packet::Frame(other));
                    }
                    Err(DecodeError::InvalidCrc) => {
                        self.buffer.drain(..1);
                        self.invalid_crc_count += 1;
                        self.desync_drop_bytes += 1;
                    }
                    Err(_) => {
                        self.buffer.drain(..1);
                        self.malformed_frame_count += 1;
                        self.desync_drop_bytes += 1;
                    }
                }

                continue;
            }

            if let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
                let prefix = self.buffer.drain(..=newline).collect::<Vec<_>>();
                packets.extend(self.text_framer.push(&prefix).into_iter().map(Packet::Text));
                continue;
            }

            if self.buffer.len() > self.max_buffer_bytes {
                let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes).max(1);
                self.buffer.drain(..drain);
                self.desync_drop_bytes += drain;
            }

            break;
        }

        packets
    }

    pub fn invalid_crc_count(&self) -> usize {
        self.invalid_crc_count
    }

    pub fn malformed_frame_count(&self) -> usize {
        self.malformed_frame_count
    }

    pub fn desync_drop_bytes(&self) -> usize {
        self.desync_drop_bytes
    }
}

pub fn encode_host_command(command: HostCommand, seq: u8) -> Vec<u8> {
    encode_host_command_with_seq(to_bmi088_command(command), seq)
}

pub fn encode_event_frame(command: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(bmi088::BMI088_HEADER_LEN + payload.len() + bmi088::BMI088_CRC_LEN);
    frame.extend_from_slice(&SOF);
    frame.push(VERSION);
    frame.push(FRAME_TYPE_EVENT);
    frame.push(command);
    frame.push(seq);
    frame.push(u8::try_from(payload.len()).unwrap_or(u8::MAX));
    frame.extend_from_slice(&payload[..usize::from(u8::try_from(payload.len()).unwrap_or(u8::MAX))]);
    let crc = bmi088::crc16_ccitt(&frame);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame
}

pub fn command_label(command: HostCommand) -> &'static str {
    bmi088::host_command_label(&to_bmi088_command(command))
}

pub fn ascii_command_bytes(command: HostCommand, newline_mode: NewlineMode) -> Vec<u8> {
    let command_text = match command {
        HostCommand::Ack => "ack",
        HostCommand::Start => "start",
        HostCommand::Stop => "stop",
        HostCommand::ReqSchema => "req_schema",
        HostCommand::ReqIdentity => "req_identity",
    };

    let mut bytes = command_text.as_bytes().to_vec();
    match newline_mode {
        NewlineMode::None => {}
        NewlineMode::Lf => bytes.push(b'\n'),
        NewlineMode::Crlf => bytes.extend_from_slice(b"\r\n"),
    }
    bytes
}

pub fn decode_frame(frame: &[u8]) -> Result<Frame, DecodeError> {
    decode_frame_with_schema(frame, None)
}

fn decode_frame_with_schema(
    frame: &[u8],
    schema: Option<&SchemaFrame>,
) -> Result<Frame, DecodeError> {
    let (frame_type, command, seq, payload): (u8, u8, u8, &[u8]) =
        decode_frame_envelope(frame).map_err(map_decode_error)?;

    match (frame_type, command) {
        (FRAME_TYPE_EVENT, CMD_IDENTITY) => match decode_binary_frame_with_schema(frame, schema)
            .map_err(map_decode_error)?
        {
            Bmi088Frame::Identity(identity) => Ok(Frame::Identity(identity)),
            _ => Err(DecodeError::Malformed("unexpected non-identity frame")),
        },
        (FRAME_TYPE_EVENT, CMD_SCHEMA) => match decode_binary_frame_with_schema(frame, schema)
            .map_err(map_decode_error)?
        {
            Bmi088Frame::Schema(schema) => Ok(Frame::Schema(schema)),
            _ => Err(DecodeError::Malformed("unexpected non-schema frame")),
        },
        (FRAME_TYPE_EVENT, CMD_SAMPLE) => {
            let raw_values = decode_sample_raw_values(payload).map_err(map_decode_error)?;
            Ok(Frame::Sample(SampleFrame { seq, raw_values }))
        }
        _ => Ok(Frame::Unknown {
            frame_type,
            command,
            seq,
            payload_len: payload.len() as u8,
        }),
    }
}

fn map_decode_error(error: Bmi088DecodeError) -> DecodeError {
    match error {
        Bmi088DecodeError::InvalidSof => DecodeError::InvalidSof,
        Bmi088DecodeError::InvalidVersion => DecodeError::InvalidVersion,
        Bmi088DecodeError::InvalidCrc => DecodeError::InvalidCrc,
        Bmi088DecodeError::SchemaMismatch(_) | Bmi088DecodeError::MalformedFrame(_) => {
            DecodeError::Malformed("invalid BMI088 frame")
        }
    }
}

fn to_bmi088_command(command: HostCommand) -> Bmi088HostCommand {
    match command {
        HostCommand::Ack => Bmi088HostCommand::Ack,
        HostCommand::Start => Bmi088HostCommand::Start,
        HostCommand::Stop => Bmi088HostCommand::Stop,
        HostCommand::ReqSchema => Bmi088HostCommand::ReqSchema,
        HostCommand::ReqIdentity => Bmi088HostCommand::ReqIdentity,
    }
}
