use eadai::bmi088_diag::{self, CommandPath, DiagStats};
use eadai::bmi088_diag::protocol::{
    CMD_IDENTITY, CMD_SAMPLE, CMD_SCHEMA, DiagnosticDecoder, Frame, HostCommand, Packet,
    decode_frame, encode_event_frame, encode_host_command,
};

#[test]
fn encodes_host_commands_with_seq_and_len_byte() {
    let frame = encode_host_command(HostCommand::Ack, 7);
    assert_eq!(&frame[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x10, 0x07, 0x00]);

    let identity = encode_host_command(HostCommand::ReqIdentity, 8);
    assert_eq!(&identity[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x14, 0x08, 0x00]);

    let shell = encode_host_command(HostCommand::ShellExec, 9);
    assert_eq!(&shell[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x28, 0x09, 0x00]);
}

#[test]
fn decoder_emits_text_schema_and_sample_packets() {
    let identity = encode_event_frame(CMD_IDENTITY, 3, &identity_payload());
    let schema = encode_event_frame(CMD_SCHEMA, 1, &schema_payload());
    let sample = encode_event_frame(CMD_SAMPLE, 2, &sample_payload(&[1, -2, 3]));
    let mut decoder = DiagnosticDecoder::new(256);
    let packets = decoder.push(&[b'o', b'k', b'\n']);
    assert!(matches!(&packets[0], Packet::Text(_)));

    let packets = decoder.push(&identity);
    match &packets[0] {
        Packet::Frame(Frame::Identity(identity)) => {
            assert_eq!(identity.device_name, "BMI088 Bringup");
            assert_eq!(identity.protocol_version, "1.0");
        }
        other => panic!("unexpected packet: {other:?}"),
    }

    let packets = decoder.push(&schema);
    match &packets[0] {
        Packet::Frame(Frame::Schema(schema)) => {
            assert_eq!(schema.rate_hz, 100);
            assert_eq!(schema.fields[0].name, "acc_x");
        }
        other => panic!("unexpected packet: {other:?}"),
    }

    let packets = decoder.push(&sample);
    match &packets[0] {
        Packet::Frame(Frame::Sample(sample)) => {
            assert_eq!(sample.raw_values, vec![1, -2, 3]);
        }
        other => panic!("unexpected packet: {other:?}"),
    }
}

#[test]
fn decode_frame_rejects_crc_mismatch() {
    let mut frame = encode_event_frame(CMD_SCHEMA, 1, &schema_payload());
    let last = frame.len() - 1;
    frame[last] ^= 0xFF;
    let error = decode_frame(&frame).expect_err("crc should fail");
    assert!(matches!(error, eadai::bmi088_diag::protocol::DecodeError::InvalidCrc));
}

#[test]
fn verdict_points_to_binary_framing_after_ascii_success() {
    let verdict = bmi088_diag::build_verdict(&DiagStats {
        sample_count: 1,
        first_sample_path: Some(CommandPath::Ascii),
        ..DiagStats::default()
    });

    assert_eq!(verdict.title, "samples-flowing");
    assert!(verdict.detail.contains("ASCII ACK/START unlocked samples"));
}

fn schema_payload() -> Vec<u8> {
    let mut payload = vec![0x01, 100, 3, 6];
    for (field_id, name) in [(0, "acc_x"), (1, "acc_y"), (2, "roll")] {
        payload.push(field_id);
        payload.push(1);
        payload.push(if name == "roll" { -2_i8 as u8 } else { 0 });
        payload.push(name.len() as u8);
        payload.push(3);
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(b"raw");
    }
    payload
}

fn sample_payload(values: &[i16]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(values.len() * 2);
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn identity_payload() -> Vec<u8> {
    let mut payload = Vec::new();

    push_tlv(&mut payload, 0x00, &[0x01]);
    push_tlv(&mut payload, 0x01, b"BMI088 Bringup");
    push_tlv(&mut payload, 0x02, b"TC264 Board");
    push_tlv(&mut payload, 0x03, b"0.4.2");
    push_tlv(&mut payload, 0x04, b"bmi088_uart4");
    push_tlv(&mut payload, 0x05, b"1.0");
    push_tlv(&mut payload, 0x06, b"uart4");
    push_tlv(&mut payload, 0x07, &100_u16.to_le_bytes());
    push_tlv(&mut payload, 0x08, &[30]);
    push_tlv(&mut payload, 0x09, &[60]);
    push_tlv(&mut payload, 0x0A, &[1]);
    push_tlv(&mut payload, 0x0B, &0x003F_u16.to_le_bytes());
    push_tlv(&mut payload, 0x0C, &115_200_u32.to_le_bytes());
    push_tlv(&mut payload, 0x0D, &[0]);

    payload
}

fn push_tlv(payload: &mut Vec<u8>, tag: u8, value: &[u8]) {
    payload.push(tag);
    payload.push(value.len() as u8);
    payload.extend_from_slice(value);
}
