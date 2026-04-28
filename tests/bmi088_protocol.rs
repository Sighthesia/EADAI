use eadai::bmi088::{
    self, Bmi088DecodeError, Bmi088Frame, Bmi088HostCommand, Bmi088SessionPhase,
    Bmi088SessionState, TelemetryPacket,
};

#[test]
fn encodes_and_decodes_identity_frames() {
    let identity = sample_identity();
    let frame = bmi088::encode_identity_frame(&identity);

    match bmi088::decode_binary_frame(&frame).expect("decode identity") {
        Bmi088Frame::Identity(decoded) => {
    assert_eq!(decoded.device_name, "BMI088 Bringup");
            assert_eq!(decoded.board_name, "TC264 Board");
            assert_eq!(decoded.protocol_version, "1.0");
            assert_eq!(decoded.sample_rate_hz, 100);
            assert_eq!(decoded.feature_flags, 0x003F);
            assert_eq!(decoded.schema_field_count, 30);
            assert_eq!(decoded.sample_payload_len, 60);
        }
        _ => panic!("expected identity frame"),
    }
}

#[test]
fn encodes_and_decodes_schema_frames() {
    let schema = bmi088::default_schema();
    let frame = bmi088::encode_schema_frame(&schema);

    match bmi088::decode_binary_frame(&frame).expect("decode schema") {
        Bmi088Frame::Schema(decoded) => {
            assert_eq!(decoded.rate_hz, 100);
            assert_eq!(decoded.fields.len(), 30);
            assert_eq!(decoded.fields[0].unit, "raw");
            assert_eq!(decoded.fields[6].scale_q, -2);
            assert_eq!(decoded.fields[6].unit, "deg");
            assert_eq!(decoded.fields[11].name, "motor_left_rear_wheel");
            assert_eq!(decoded.sample_len, 60);
        }
        _ => panic!("expected schema frame"),
    }
}

#[test]
fn decodes_legacy_schema_payload_without_header() {
    let payload = legacy_schema_payload(&[
        (0, "acc_x", "raw"),
        (0, "acc_y", "raw"),
        (-2, "roll", "deg"),
        (-2, "pid_yaw_d_gain", ""),
    ]);

    let schema = bmi088::decode_schema_payload(&payload).expect("decode legacy schema");

    assert_eq!(schema.schema_version, bmi088::BMI088_SCHEMA_VERSION);
    assert_eq!(schema.rate_hz, 100);
    assert_eq!(schema.fields.len(), 4);
    assert_eq!(schema.sample_len, 8);
    assert_eq!(schema.fields[0].name, "acc_x");
    assert_eq!(schema.fields[2].scale_q, -2);
    assert_eq!(schema.fields[2].unit, "deg");
    assert_eq!(schema.fields[3].name, "pid_yaw_d_gain");
}

#[test]
fn decodes_mixed_legacy_schema_payload_from_device_shape() {
    let payload = mixed_legacy_schema_payload(
        (0, "pid_yaw_i_gain", ""),
        &[
            (0x1B, 1, 0, "pid_tuning_mode", ""),
            (0x1C, 1, 0, "output_limit", ""),
            (0x1D, 1, 0, "bench_test_throttle", ""),
        ],
    );

    let schema = bmi088::decode_schema_payload(&payload).expect("decode mixed legacy schema");

    assert_eq!(schema.fields.len(), 4);
    assert_eq!(schema.sample_len, 8);
    assert_eq!(schema.fields[0].name, "pid_yaw_i_gain");
    assert_eq!(schema.fields[1].field_id, 0x1B);
    assert_eq!(schema.fields[1].name, "pid_tuning_mode");
    assert_eq!(schema.fields[2].name, "output_limit");
    assert_eq!(schema.fields[3].name, "bench_test_throttle");
}

#[test]
fn encodes_and_decodes_sample_frames() {
    let sample = bmi088::default_sample(3);
    let frame = bmi088::encode_sample_frame(&sample);

    match bmi088::decode_binary_frame(&frame).expect("decode sample") {
        Bmi088Frame::Sample(decoded) => {
            assert_eq!(decoded.fields.len(), 30);
            assert_eq!(decoded.fields[0].name, "acc_x");
            assert_eq!(decoded.fields[29].name, "bench_test_throttle");
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

    let identity_commands = session.on_frame(&Bmi088Frame::Identity(sample_identity()));
    assert!(identity_commands.is_empty());

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
fn boot_commands_are_only_emitted_once_per_session_start() {
    let mut session = Bmi088SessionState::new();

    assert_eq!(session.boot_commands(), vec![Bmi088HostCommand::ReqSchema]);
    assert!(session.boot_commands().is_empty());
    assert_eq!(session.phase(), Bmi088SessionPhase::AwaitingSchema);
}

#[test]
fn schema_retry_commands_are_available_until_handshake_advances() {
    let mut session = Bmi088SessionState::new();

    session.boot_commands();
    assert_eq!(session.schema_retry_commands(), vec![Bmi088HostCommand::ReqSchema]);

    session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema()));
    assert!(session.schema_retry_commands().is_empty());
}

#[test]
fn host_handshake_commands_encode_to_binary_request_frames() {
    let req_identity = bmi088::encode_host_command(Bmi088HostCommand::ReqIdentity);
    let req_schema = bmi088::encode_host_command(Bmi088HostCommand::ReqSchema);
    let req_tuning = bmi088::encode_host_command(Bmi088HostCommand::ReqTuning);
    let set_tuning = bmi088::encode_host_command_with_payload(Bmi088HostCommand::SetTuning, b"payload");
    let shell_exec = bmi088::encode_host_command_with_payload(Bmi088HostCommand::ShellExec, b"help");
    let ack = bmi088::encode_host_command(Bmi088HostCommand::Ack);
    let start = bmi088::encode_host_command(Bmi088HostCommand::Start);

    assert_eq!(&req_identity[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x14, 0x00, 0x00]);
    assert_eq!(&req_schema[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x13, 0x00, 0x00]);
    assert_eq!(&req_tuning[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x26, 0x00, 0x00]);
    assert_eq!(&set_tuning[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x27, 0x00, 0x07]);
    assert_eq!(&shell_exec[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x28, 0x00, 0x04]);
    assert_eq!(&ack[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x10, 0x00, 0x00]);
    assert_eq!(&start[..7], &[0xA5, 0x5A, 0x01, 0x01, 0x11, 0x00, 0x00]);
}

#[test]
fn shell_command_and_output_frames_round_trip() {
    let output = bmi088::encode_shell_output_frame(
        &eadai::message::LinePayload {
            text: "ok".to_string(),
            raw: b"ok".to_vec(),
        },
        12,
    );

    match bmi088::decode_binary_frame(&output).expect("decode shell output") {
        Bmi088Frame::ShellOutput(decoded) => assert_eq!(decoded.text, "ok"),
        _ => panic!("expected shell output frame"),
    }
}

#[test]
fn decodes_response_typed_frames_for_backward_compatibility() {
    let identity = response_frame(bmi088::BMI088_CMD_IDENTITY, 7, &sample_identity().encode_payload());
    let schema = response_frame(bmi088::BMI088_CMD_SCHEMA, 8, &bmi088::default_schema().encode_payload());
    let sample = response_frame(bmi088::BMI088_CMD_SAMPLE, 9, &sample_payload_from_raw(&[1, 2, 3]));

    assert!(matches!(
        bmi088::decode_binary_frame(&identity).expect("decode response identity"),
        Bmi088Frame::Identity(_)
    ));
    assert!(matches!(
        bmi088::decode_binary_frame(&schema).expect("decode response schema"),
        Bmi088Frame::Schema(_)
    ));
    assert!(matches!(
        bmi088::decode_binary_frame_with_schema(&sample, Some(&three_field_schema())).expect("decode response sample"),
        Bmi088Frame::Sample(_)
    ));
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
fn duplicate_schema_frames_do_not_restart_streaming_handshake() {
    let mut session = Bmi088SessionState::new();

    session.boot_commands();
    session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema()));
    session.on_host_command(Bmi088HostCommand::Ack);
    session.on_host_command(Bmi088HostCommand::Start);
    assert_eq!(session.phase(), Bmi088SessionPhase::Streaming);

    assert!(session.on_frame(&Bmi088Frame::Schema(bmi088::default_schema())).is_empty());
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

#[test]
fn stream_decoder_does_not_replace_streaming_schema_with_short_tuning_schema_when_identity_disagrees() {
    let identity = sample_identity();
    let stream_schema = bmi088::default_schema();
    let tuning_schema = bmi088::decode_schema_payload(&mixed_legacy_schema_payload(
        (0, "pid_yaw_i_gain", ""),
        &[
            (0x1B, 1, 0, "pid_yaw_d_gain", ""),
            (0x1C, 1, 0, "pid_output_limit", ""),
            (0x1D, 1, 0, "pid_tuning_test_throttle", ""),
        ],
    ))
    .expect("decode tuning schema");
    let sample = bmi088::default_sample(1);
    let mut decoder = bmi088::Bmi088StreamDecoder::new(4096);

    let identity_packets = decoder.push(&bmi088::encode_identity_frame(&identity));
    assert!(matches!(identity_packets.first(), Some(TelemetryPacket::Identity(_))));

    let stream_schema_packets = decoder.push(&bmi088::encode_schema_frame(&stream_schema));
    assert!(matches!(stream_schema_packets.first(), Some(TelemetryPacket::Schema(_))));

    let tuning_schema_packets = decoder.push(&bmi088::encode_schema_frame(&tuning_schema));
    assert!(matches!(tuning_schema_packets.first(), Some(TelemetryPacket::Schema(_))));

    let sample_packets = decoder.push(&bmi088::encode_sample_frame(&sample));
    match sample_packets.first().expect("sample packet") {
        TelemetryPacket::Sample(decoded) => {
            assert_eq!(decoded.fields.len(), 30);
            assert_eq!(decoded.fields[0].name, "acc_x");
            assert_eq!(decoded.fields[29].name, "bench_test_throttle");
        }
        _ => panic!("expected sample packet"),
    }
}

#[test]
fn stream_decoder_falls_back_to_default_sample_schema_when_cached_schema_is_short() {
    let tuning_schema = bmi088::decode_schema_payload(&mixed_legacy_schema_payload(
        (0, "pid_yaw_i_gain", ""),
        &[
            (0x1B, 1, 0, "pid_yaw_d_gain", ""),
            (0x1C, 1, 0, "pid_output_limit", ""),
            (0x1D, 1, 0, "pid_tuning_test_throttle", ""),
        ],
    ))
    .expect("decode tuning schema");
    let sample = bmi088::default_sample(2);
    let mut decoder = bmi088::Bmi088StreamDecoder::new(4096);

    let schema_packets = decoder.push(&bmi088::encode_schema_frame(&tuning_schema));
    assert!(matches!(schema_packets.first(), Some(TelemetryPacket::Schema(_))));

    let sample_packets = decoder.push(&bmi088::encode_sample_frame(&sample));
    match sample_packets.first().expect("sample packet") {
        TelemetryPacket::Sample(decoded) => {
            assert_eq!(decoded.fields.len(), 30);
            assert_eq!(decoded.fields[0].name, "acc_x");
            assert_eq!(decoded.fields[29].name, "bench_test_throttle");
        }
        _ => panic!("expected sample packet"),
    }
}

#[test]
fn stream_decoder_emits_identity_packets_before_schema() {
    let mut decoder = bmi088::Bmi088StreamDecoder::new(4096);
    let identity_packets = decoder.push(&bmi088::encode_identity_frame(&sample_identity()));
    assert!(matches!(
        identity_packets.first(),
        Some(TelemetryPacket::Identity(identity)) if identity.device_name == "BMI088 Bringup"
    ));

    let schema_packets = decoder.push(&bmi088::encode_schema_frame(&bmi088::default_schema()));
    assert!(matches!(schema_packets.first(), Some(TelemetryPacket::Schema(_))));
}

fn sample_identity() -> bmi088::Bmi088IdentityFrame {
    bmi088::Bmi088IdentityFrame {
        seq: 4,
        identity_format_version: 1,
        device_name: "BMI088 Bringup".to_string(),
        board_name: "TC264 Board".to_string(),
        firmware_version: "0.4.2".to_string(),
        protocol_name: "bmi088_uart4".to_string(),
        protocol_version: "1.0".to_string(),
        transport_name: "uart4".to_string(),
        sample_rate_hz: 100,
        schema_field_count: 30,
        sample_payload_len: 60,
        protocol_version_byte: 1,
        feature_flags: 0x003F,
        baud_rate: 115_200,
        protocol_minor_version: 0,
    }
}

fn legacy_schema_payload(fields: &[(i8, &str, &str)]) -> Vec<u8> {
    let mut payload = Vec::new();

    for (scale_q, name, unit) in fields {
        payload.push(*scale_q as u8);
        payload.push(name.len() as u8);
        payload.push(unit.len() as u8);
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(unit.as_bytes());
    }

    payload
}

fn mixed_legacy_schema_payload(
    first_field: (i8, &str, &str),
    remaining_fields: &[(u8, u8, i8, &str, &str)],
) -> Vec<u8> {
    let mut payload = Vec::new();
    let (scale_q, name, unit) = first_field;
    payload.push(scale_q as u8);
    payload.push(name.len() as u8);
    payload.push(unit.len() as u8);
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(unit.as_bytes());

    for (field_id, field_type, scale_q, name, unit) in remaining_fields {
        payload.push(*field_id);
        payload.push(*field_type);
        payload.push(*scale_q as u8);
        payload.push(name.len() as u8);
        payload.push(unit.len() as u8);
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(unit.as_bytes());
    }

    payload
}

fn response_frame(command: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
    encode_frame_like_device(bmi088::BMI088_FRAME_TYPE_RESPONSE, command, seq, payload)
}

fn encode_frame_like_device(frame_type: u8, command: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(bmi088::BMI088_HEADER_LEN + payload.len() + bmi088::BMI088_CRC_LEN);
    frame.extend_from_slice(&bmi088::BMI088_SOF);
    frame.push(bmi088::BMI088_VERSION);
    frame.push(frame_type);
    frame.push(command);
    frame.push(seq);
    frame.push(payload.len() as u8);
    frame.extend_from_slice(payload);
    let crc = bmi088::crc16_ccitt(&frame);
    frame.extend_from_slice(&crc.to_le_bytes());
    frame
}

fn sample_payload_from_raw(values: &[i16]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(values.len() * 2);
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn three_field_schema() -> bmi088::Bmi088SchemaFrame {
    bmi088::Bmi088SchemaFrame {
        seq: 0,
        schema_version: bmi088::BMI088_SCHEMA_VERSION,
        rate_hz: 100,
        sample_len: 6,
        fields: vec![
            bmi088::Bmi088FieldDescriptor {
                field_id: 0,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "acc_x".to_string(),
                unit: "raw".to_string(),
                scale_q: 0,
            },
            bmi088::Bmi088FieldDescriptor {
                field_id: 1,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "acc_y".to_string(),
                unit: "raw".to_string(),
                scale_q: 0,
            },
            bmi088::Bmi088FieldDescriptor {
                field_id: 2,
                field_type: bmi088::BMI088_FIELD_TYPE_I16,
                name: "roll".to_string(),
                unit: "deg".to_string(),
                scale_q: -2,
            },
        ],
    }
}
