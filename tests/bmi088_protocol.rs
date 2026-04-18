use eadai::bmi088::{
    self, Bmi088DecodeError, Bmi088Frame, Bmi088HostCommand, Bmi088SessionPhase,
    Bmi088SessionState, TelemetryPacket,
};

#[test]
fn encodes_and_decodes_schema_frames() {
    let schema = bmi088::default_schema();
    let frame = bmi088::encode_schema_frame(&schema);

    match bmi088::decode_binary_frame(&frame).expect("decode schema") {
        Bmi088Frame::Schema(decoded) => {
            assert_eq!(decoded.rate_hz, 100);
            assert_eq!(decoded.fields.len(), 9);
            assert_eq!(decoded.fields[6].scale_q, -2);
            assert_eq!(decoded.fields[6].unit, "deg");
        }
        _ => panic!("expected schema frame"),
    }
}

#[test]
fn encodes_and_decodes_sample_frames() {
    let sample = bmi088::default_sample(3);
    let frame = bmi088::encode_sample_frame(&sample);

    match bmi088::decode_binary_frame(&frame).expect("decode sample") {
        Bmi088Frame::Sample(decoded) => {
            assert_eq!(decoded.fields.len(), 9);
            assert_eq!(decoded.fields[0].name, "acc_x");
        }
        _ => panic!("expected sample frame"),
    }
}

#[test]
fn validates_crc_and_resynchronizes_bad_frames() {
    let mut frame = bmi088::encode_schema_frame(&bmi088::default_schema());
    let last = frame.len() - 1;
    frame[last] ^= 0xFF;

    assert!(matches!(
        bmi088::decode_binary_frame(&frame),
        Err(Bmi088DecodeError::InvalidCrc)
    ));

    let mut decoder = bmi088::Bmi088StreamDecoder::new(4096);
    let mut bytes = vec![0x00, 0xAA, 0x55];
    bytes.extend_from_slice(&bmi088::encode_schema_frame(&bmi088::default_schema()));

    let packets = decoder.push(&bytes);
    assert!(packets
        .iter()
        .any(|packet| matches!(packet, TelemetryPacket::Schema(_))));
}

#[test]
fn session_flow_requests_schema_then_ack_and_start() {
    let mut session = Bmi088SessionState::new();
    let boot = session.boot_commands();
    assert_eq!(boot.len(), 1);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingSchema);

    let commands = session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema()));
    assert_eq!(commands.len(), 2);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingAck);

    session.on_host_command(Bmi088HostCommand::Ack);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingStart);
    session.on_host_command(Bmi088HostCommand::Start);
    assert_eq!(session.phase(), Bmi088SessionPhase::Streaming);
}
