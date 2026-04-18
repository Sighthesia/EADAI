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
            assert_eq!(decoded.fields[0].unit, "raw");
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
    assert!(
        packets
            .iter()
            .any(|packet| matches!(packet, TelemetryPacket::Schema(_)))
    );
}

#[test]
fn session_flow_requests_schema_then_ack_and_start() {
    let mut session = Bmi088SessionState::new();
    let boot = session.boot_commands();
    assert_eq!(boot.len(), 1);
    assert_eq!(boot[0], Bmi088HostCommand::ReqSchema);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingSchema);

    let commands = session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema()));
    assert_eq!(commands.len(), 2);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingAck);
    assert_eq!(commands[0], Bmi088HostCommand::Ack);
    assert_eq!(commands[1], Bmi088HostCommand::Start);

    session.on_host_command(Bmi088HostCommand::Ack);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingStart);
    session.on_host_command(Bmi088HostCommand::Start);
    assert_eq!(session.phase(), Bmi088SessionPhase::Streaming);
}

#[test]
fn host_handshake_commands_encode_to_binary_request_frames() {
    let req_schema = bmi088::encode_host_command(Bmi088HostCommand::ReqSchema);
    let ack = bmi088::encode_host_command(Bmi088HostCommand::Ack);
    let start = bmi088::encode_host_command(Bmi088HostCommand::Start);

    assert_eq!(&req_schema[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x13, 0x00, 0x00]);
    assert_eq!(&ack[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x10, 0x00, 0x00]);
    assert_eq!(&start[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x11, 0x00, 0x00]);
}

#[test]
fn schema_rehandshake_requires_ack_then_start_again() {
    let mut session = Bmi088SessionState::new();

    session.on_host_command(Bmi088HostCommand::ReqSchema);
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingSchema);

    let commands = session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema()));
    assert_eq!(
        commands,
        vec![Bmi088HostCommand::Ack, Bmi088HostCommand::Start]
    );
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingAck);

    session.on_host_command(Bmi088HostCommand::Ack);
    session.on_host_command(Bmi088HostCommand::Start);
    assert_eq!(session.phase(), Bmi088SessionPhase::Streaming);
}

#[test]
fn stream_decoder_uses_latest_schema_order_for_samples() {
    let schema = bmi088::Bmi088SchemaFrame {
        seq: 0,
        schema_version: bmi088::BMI088_SCHEMA_VERSION,
        rate_hz: 100,
        sample_len: 6,
        fields: vec![
            bmi088::Bmi088FieldDescriptor {
                field_id: 0,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "yaw".to_string(),
                unit: "deg".to_string(),
                scale_q: -2,
            },
            bmi088::Bmi088FieldDescriptor {
                field_id: 1,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "pitch".to_string(),
                unit: "deg".to_string(),
                scale_q: -2,
            },
            bmi088::Bmi088FieldDescriptor {
                field_id: 2,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "roll".to_string(),
                unit: "deg".to_string(),
                scale_q: -2,
            },
        ],
    };
    let sample = bmi088::Bmi088SampleFrame::from_raw_values(&schema, &[100, 200, 300])
        .expect("sample matches schema");
    let mut decoder = bmi088::Bmi088StreamDecoder::new(4096);

    let schema_packets = decoder.push(&bmi088::encode_schema_frame(&schema));
    assert!(matches!(schema_packets.first(), Some(TelemetryPacket::Schema(_))));

    let sample_packets = decoder.push(&bmi088::encode_sample_frame(&sample));
    match sample_packets.first().expect("sample packet") {
        TelemetryPacket::Sample(decoded) => {
            assert_eq!(decoded.fields[0].name, "yaw");
            assert_eq!(decoded.fields[1].name, "pitch");
            assert_eq!(decoded.fields[2].name, "roll");
        }
        _ => panic!("expected sample packet"),
    }
}
