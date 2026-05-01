/// CRTP adapter for the self-describing protocol.
///
/// This module provides the mapping between self-describing protocol frames
/// and CRTP packets, allowing the generic protocol to be transported over
/// CRTP links while keeping the protocol model transport-agnostic.
use super::codec::{DecodeError, decode_frame, encode_frame};
use super::frame::Frame;
use crate::protocols::crtp::{CrtpPacket, CrtpPort};

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
}
