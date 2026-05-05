/// CRTP adapter for the self-describing protocol.
///
/// This module provides the mapping between self-describing protocol frames
/// and CRTP packets, allowing the generic protocol to be transported over
/// CRTP links while keeping the protocol model transport-agnostic.
use super::codec::{DecodeError, decode_frame, encode_frame};
use super::frame::Frame;
use super::state::HandshakeState;
use crate::protocols::crtp::{CrtpPacket, CrtpPort};

const RAW_PREVIEW_BYTES: usize = 24;

/// Optional runtime context for classifying raw self-describing decode failures.
#[derive(Clone, Debug)]
pub struct RawSelfDescribingDecodeContext {
    /// Current handshake state, if known.
    pub handshake_state: HandshakeState,
    /// Whether the session has already entered streaming.
    pub is_streaming: bool,
}

impl RawSelfDescribingDecodeContext {
    /// Return a bounded phase label for log output.
    pub fn phase_label(&self) -> &'static str {
        match self.handshake_state {
            HandshakeState::WaitingIdentity => "before-handshake",
            HandshakeState::WaitingCommandCatalog
            | HandshakeState::WaitingVariableCatalog
            | HandshakeState::WaitingHostAck
            | HandshakeState::ReadyToStream => "post-handshake",
            HandshakeState::Streaming => "streaming",
            HandshakeState::Error(_) => "handshake-error",
        }
    }

    /// Return whether a failure happened after the canonical handshake.
    pub fn is_post_handshake(&self) -> bool {
        matches!(
            self.handshake_state,
            HandshakeState::WaitingCommandCatalog
                | HandshakeState::WaitingVariableCatalog
                | HandshakeState::WaitingHostAck
                | HandshakeState::ReadyToStream
                | HandshakeState::Streaming
        ) || self.is_streaming
    }
}

/// Compact diagnosis for raw self-describing decode failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawSelfDescribingDecodeFailure {
    /// Log phase label.
    pub phase: &'static str,
    /// First payload byte, if any.
    pub first_payload_byte: Option<u8>,
    /// Payload length in bytes.
    pub payload_len: usize,
    /// Optional bounded hint string.
    pub hint: Option<&'static str>,
}

/// Streaming outcome produced by the raw self-describing decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawSelfDescribingDecodeOutcome {
    /// A decoded frame.
    Frame(Frame),
    /// A bounded decode failure with diagnostic context.
    Failure(RawSelfDescribingDecodeFailure),
}

/// Classify a raw self-describing decode failure for logs.
pub fn classify_raw_self_describing_decode_failure(
    payload: &[u8],
    context: Option<&RawSelfDescribingDecodeContext>,
    error: &DecodeError,
) -> RawSelfDescribingDecodeFailure {
    let phase = context.map(|ctx| ctx.phase_label()).unwrap_or("unscoped");
    let first_payload_byte = payload.first().copied();
    let hint = match (context, payload.first(), error) {
        (Some(ctx), Some(0x00), DecodeError::InvalidFrameType(0)) if ctx.is_post_handshake() => {
            Some("likely bare telemetry sample payload missing frame type 0x05")
        }
        (Some(ctx), Some(0x01..=0x03), DecodeError::InvalidFrameType(_)) if ctx.is_post_handshake() => {
            Some("likely repeated identity or catalog retransmit after ACK")
        }
        (Some(ctx), Some(0x05), DecodeError::TruncatedData) if ctx.is_post_handshake() => {
            Some("likely truncated telemetry sample payload after frame type 0x05")
        }
        _ => None,
    };

    RawSelfDescribingDecodeFailure {
        phase,
        first_payload_byte,
        payload_len: payload.len(),
        hint,
    }
}

/// CRTP port used for the self-describing protocol.
/// Using Debug port (0x7) which is not heavily used by standard Crazyflie subsystems.
pub const SELF_DESCRIBING_CRTP_PORT: u8 = 0x07;

/// CRTP channel used for the self-describing protocol.
/// Channel 3 is chosen to avoid conflict with debug text output on channel 0.
pub const SELF_DESCRIBING_CRTP_CHANNEL: u8 = 3;

/// Error type for CRTP adapter operations.
#[derive(Debug)]
pub enum CrtpAdapterError {
    /// The CRTP packet is not on the self-describing protocol port/channel.
    WrongPort,
    /// Failed to decode the self-describing protocol frame.
    DecodeError(DecodeError),
    /// The payload is too short to contain a valid frame.
    PayloadTooShort,
}

impl std::fmt::Display for CrtpAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongPort => write!(f, "not a self-describing protocol packet"),
            Self::DecodeError(e) => write!(f, "decode error: {e}"),
            Self::PayloadTooShort => write!(f, "payload too short"),
        }
    }
}

impl std::error::Error for CrtpAdapterError {}

impl From<DecodeError> for CrtpAdapterError {
    fn from(e: DecodeError) -> Self {
        Self::DecodeError(e)
    }
}

/// Check if a CRTP packet is for the self-describing protocol.
pub fn is_self_describing_packet(packet: &CrtpPacket) -> bool {
    // CRTP port 0x7 is decoded as CrtpPort::Debug by the standard decoder.
    // We use this port for the self-describing protocol.
    matches!(packet.port, CrtpPort::Debug) && packet.channel == SELF_DESCRIBING_CRTP_CHANNEL
}

/// Try to decode a CRTP packet as a self-describing protocol frame.
///
/// Returns `Ok(Some(frame))` if successful, `Ok(None)` if the packet is not
/// for the self-describing protocol, or `Err` if decoding failed.
pub fn decode_crtp_packet(packet: &CrtpPacket) -> Result<Option<Frame>, CrtpAdapterError> {
    if !is_self_describing_packet(packet) {
        return Ok(None);
    }

    if packet.payload.is_empty() {
        return Err(CrtpAdapterError::PayloadTooShort);
    }

    let frame = decode_frame(&packet.payload)?;
    Ok(Some(frame))
}

/// Encode a self-describing protocol frame as a CRTP packet.
pub fn encode_crtp_packet(frame: &Frame) -> CrtpPacket {
    let payload = encode_frame(frame);
    CrtpPacket {
        port: CrtpPort::Debug,
        channel: SELF_DESCRIBING_CRTP_CHANNEL,
        payload,
    }
}

/// Encode a self-describing protocol frame using the raw outer transport.
pub fn encode_raw_transport_frame(frame: &Frame) -> Vec<u8> {
    let payload = encode_frame(frame);
    let mut raw = vec![0x73, payload.len() as u8];
    raw.extend_from_slice(&payload);
    raw
}

/// Streaming decoder for the raw self-describing outer transport.
pub struct RawSelfDescribingDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
}

impl RawSelfDescribingDecoder {
    /// Create a new decoder with bounded buffering.
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
        }
    }

    /// Push raw transport bytes and return any decoded frames.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Frame> {
        self.push_with_context(chunk, None)
            .into_iter()
            .filter_map(|outcome| match outcome {
                RawSelfDescribingDecodeOutcome::Frame(frame) => Some(frame),
                RawSelfDescribingDecodeOutcome::Failure(_) => None,
            })
            .collect()
    }

    /// Push raw transport bytes with optional runtime context for diagnostics.
    pub fn push_with_context(
        &mut self,
        chunk: &[u8],
        context: Option<&RawSelfDescribingDecodeContext>,
    ) -> Vec<RawSelfDescribingDecodeOutcome> {
        self.buffer.extend_from_slice(chunk);
        let mut outcomes = Vec::new();

        loop {
            if self.buffer.len() < 2 {
                break;
            }

            if self.buffer[0] != 0x73 {
                self.buffer.drain(..1);
                if self.buffer.len() > self.max_buffer_bytes {
                    let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes);
                    self.buffer.drain(..drain.max(1));
                }
                continue;
            }

            let payload_len = self.buffer[1] as usize;
            let frame_len = 2 + payload_len;

            if self.buffer.len() < frame_len {
                break;
            }

            let payload = self.buffer[2..frame_len].to_vec();
            match decode_frame(&payload) {
                Ok(frame) => {
                    outcomes.push(RawSelfDescribingDecodeOutcome::Frame(frame));
                    self.buffer.drain(..frame_len);
                }
                Err(error) => {
                    let diagnosis = classify_raw_self_describing_decode_failure(
                        &payload,
                        context,
                        &error,
                    );
                    eprintln!(
                        "[self-describing][raw][{}] decode failed error={} payload_len={} preview={}{}",
                        diagnosis.phase,
                        error,
                        payload.len(),
                        hex_preview(&payload, RAW_PREVIEW_BYTES),
                        diagnosis
                            .hint
                            .map(|hint| format!(" hint={hint}"))
                            .unwrap_or_default(),
                    );
                    outcomes.push(RawSelfDescribingDecodeOutcome::Failure(diagnosis));
                    self.buffer.drain(..frame_len);
                }
            }

            if self.buffer.len() > self.max_buffer_bytes {
                let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes);
                self.buffer.drain(..drain.max(1));
            }
        }

        outcomes
    }
}

fn hex_preview(bytes: &[u8], limit: usize) -> String {
    let mut parts = bytes
        .iter()
        .take(limit)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>();
    if bytes.len() > limit {
        parts.push("...".to_string());
    }
    parts.join(" ")
}

/// Get the CRTP port label for the self-describing protocol.
pub fn self_describing_port_label() -> &'static str {
    "self_describing"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::self_describing::frame::*;

    #[test]
    fn test_is_self_describing_packet() {
        let packet = CrtpPacket {
            port: CrtpPort::Debug,
            channel: SELF_DESCRIBING_CRTP_CHANNEL,
            payload: vec![0x01],
        };
        assert!(is_self_describing_packet(&packet));

        let wrong_port = CrtpPacket {
            port: CrtpPort::Console,
            channel: 0,
            payload: vec![0x01],
        };
        assert!(!is_self_describing_packet(&wrong_port));

        let wrong_channel = CrtpPacket {
            port: CrtpPort::Debug,
            channel: 0, // Channel 0 is debug text, not self-describing
            payload: vec![0x01],
        };
        assert!(!is_self_describing_packet(&wrong_channel));
    }

    #[test]
    fn test_decode_crtp_packet_roundtrip() {
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
        let packet = encode_crtp_packet(&frame);

        let decoded = decode_crtp_packet(&packet).expect("decode should succeed");
        match decoded {
            Some(Frame::Identity(d)) => {
                assert_eq!(d.device_name, "Test Device");
                assert_eq!(d.sample_rate_hz, 100);
            }
            _ => panic!("expected identity frame"),
        }
    }

    #[test]
    fn test_decode_non_self_describing_packet() {
        let packet = CrtpPacket {
            port: CrtpPort::Console,
            channel: 0,
            payload: vec![0x01, 0x02],
        };
        let result = decode_crtp_packet(&packet).expect("should not error");
        assert!(result.is_none());
    }

    #[test]
    fn test_decode_empty_payload() {
        let packet = CrtpPacket {
            port: CrtpPort::Debug,
            channel: SELF_DESCRIBING_CRTP_CHANNEL,
            payload: vec![],
        };
        assert!(matches!(
            decode_crtp_packet(&packet),
            Err(CrtpAdapterError::PayloadTooShort)
        ));
    }

    #[test]
    fn test_raw_decoder_drops_entire_invalid_frame_before_resyncing() {
        let mut decoder = RawSelfDescribingDecoder::new(64);

        let invalid = vec![0x73, 0x03, 0x00, 0xAA, 0xBB];
        let valid = encode_raw_transport_frame(&Frame::HostAck(HostAck {
            stage: AckStage::Identity,
        }));

        let mut chunk = invalid;
        chunk.extend_from_slice(&valid);

        let frames = decoder.push(&chunk);
        assert_eq!(frames.len(), 1);
        assert!(matches!(frames[0], Frame::HostAck(HostAck { stage: AckStage::Identity })));
    }

    #[test]
    fn test_raw_decode_failure_classification_flags_bare_sample_payloads() {
        let context = RawSelfDescribingDecodeContext {
            handshake_state: HandshakeState::Streaming,
            is_streaming: true,
        };
        let diagnosis = classify_raw_self_describing_decode_failure(
            &[0x00, 0x83, 0x00],
            Some(&context),
            &DecodeError::InvalidFrameType(0),
        );

        assert_eq!(diagnosis.phase, "streaming");
        assert_eq!(diagnosis.first_payload_byte, Some(0x00));
        assert_eq!(diagnosis.payload_len, 3);
        assert_eq!(diagnosis.hint, Some("likely bare telemetry sample payload missing frame type 0x05"));
    }

    #[test]
    fn test_raw_decode_failure_classification_flags_retransmits_after_ack() {
        let context = RawSelfDescribingDecodeContext {
            handshake_state: HandshakeState::WaitingHostAck,
            is_streaming: false,
        };
        let diagnosis = classify_raw_self_describing_decode_failure(
            &[0x03, 0x00, 0x01],
            Some(&context),
            &DecodeError::InvalidFrameType(3),
        );

        assert_eq!(diagnosis.phase, "post-handshake");
        assert_eq!(diagnosis.hint, Some("likely repeated identity or catalog retransmit after ACK"));
    }
}
