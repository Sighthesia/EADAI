/// BMI088 protocol module — organized by domain concern.
///
/// - `constants`: protocol wire constants and default field definitions
/// - `models`: data types, frame models, error type
/// - `encoder`: frame encoding, CRC, host command encoding, default generators
/// - `decoder`: frame decoding, stream decoder, identity/schema/sample parsing
/// - `session`: handshake state machine
mod constants;
mod decoder;
mod encoder;
mod models;
mod session;

// Re-export constants
pub use constants::*;

// Re-export models and error
pub use models::*;

// Re-export encoder public API
pub use encoder::{
    crc16_ccitt, default_sample, default_schema, encode_host_command,
    encode_host_command_with_payload, encode_host_command_with_seq,
    encode_host_command_with_seq_and_payload, encode_identity_frame,
    encode_identity_frame_with_seq, encode_sample_frame, encode_sample_frame_with_seq,
    encode_schema_frame, encode_schema_frame_with_seq, encode_shell_output_frame,
    host_command_from_text, host_command_label, scale_raw,
};

// Re-export decoder public API
pub use decoder::{
    Bmi088StreamDecoder, decode_binary_frame, decode_binary_frame_with_schema,
    decode_frame_envelope, decode_identity_payload, decode_identity_payload_with_seq,
    decode_sample_payload, decode_sample_payload_with_schema,
    decode_sample_payload_with_schema_and_seq, decode_sample_raw_values, decode_schema_payload,
    decode_schema_payload_with_seq, find_sof, frame_len, frame_len_from_payload_len,
};

// Re-export session
pub use session::Bmi088SessionState;
