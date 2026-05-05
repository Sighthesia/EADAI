/// Generic self-describing device protocol.
///
/// This module implements a transport-agnostic protocol that allows devices
/// to declare their identity, command catalog, and variable catalog during
/// handshake, then stream telemetry samples with bitmap compression for
/// unchanged fields.
pub mod bitmap;
pub mod codec;
pub mod crtp_adapter;
pub mod frame;
pub mod session;
pub mod state;

pub use bitmap::BitmapCodec;
pub use codec::{DecodeError, decode_frame, encode_frame};
pub use crtp_adapter::{
    CrtpAdapterError, RawSelfDescribingDecodeContext, RawSelfDescribingDecodeFailure,
    RawSelfDescribingDecodeOutcome, RawSelfDescribingDecoder, SELF_DESCRIBING_CRTP_CHANNEL,
    SELF_DESCRIBING_CRTP_PORT, classify_raw_self_describing_decode_failure, decode_crtp_packet,
    encode_crtp_packet, encode_raw_transport_frame, is_self_describing_packet,
    self_describing_port_label,
};
pub use frame::*;
pub use session::{
    SelfDescribingSession, SelfDescribingStreamingDriftEvidence,
    SelfDescribingStreamingDriftVerdict,
};
pub use state::{HandshakeMachine, HandshakeState};
