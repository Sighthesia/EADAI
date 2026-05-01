use eadai::protocols::self_describing::{
    crtp_adapter::{
        SELF_DESCRIBING_CRTP_CHANNEL, SELF_DESCRIBING_CRTP_PORT, encode_crtp_packet,
        is_self_describing_packet,
    },
    frame::*,
    session::SelfDescribingSession,
};

/// Helper to build a raw CRTP frame for testing.
fn build_raw_crtp_frame(payload: &[u8]) -> Vec<u8> {
    let header = ((SELF_DESCRIBING_CRTP_PORT & 0x07) << 5) | (SELF_DESCRIBING_CRTP_CHANNEL & 0x03);
    let mut frame = vec![header, payload.len() as u8];
    frame.extend_from_slice(payload);
    let crc = crc8_update(0, &frame);
    frame.push(crc);
    frame
}

fn crc8_update(crc: u8, data: &[u8]) -> u8 {
    let mut crc = crc;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x1D;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[test]
fn test_session_generates_host_ack_for_variable_catalog() {
    let mut session = SelfDescribingSession::new();

    // Complete handshake up to variable catalog
    session.on_frame(&Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Test Device".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    }));

    session.on_frame(&Frame::CommandCatalogPage(CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![],
    }));

    // Receive variable catalog - should generate HostAck
    let responses = session.on_frame(&Frame::VariableCatalogPage(VariableCatalogPage {
        page: 0,
        total_pages: 1,
        variables: vec![
            VariableDescriptor {
                name: "acc_x".to_string(),
                order: 0,
                unit: "m/s^2".to_string(),
                adjustable: false,
                value_type: ValueType::I16,
            },
            VariableDescriptor {
                name: "gain".to_string(),
                order: 1,
                unit: "".to_string(),
                adjustable: true,
                value_type: ValueType::F32,
            },
        ],
    }));

    // Should have one response: HostAck
    assert_eq!(responses.len(), 1);
    match &responses[0] {
        Frame::HostAck(ack) => {
            assert_eq!(ack.stage, AckStage::VariableCatalog);
            assert_eq!(ack.status, 0);
            assert_eq!(ack.message, "OK");
        }
        _ => panic!("expected HostAck response"),
    }

    // Encode the response as CRTP
    let crtp_packet = encode_crtp_packet(&responses[0]);
    assert_eq!(crtp_packet.port, eadai::protocols::crtp::CrtpPort::Debug);
    assert_eq!(crtp_packet.channel, SELF_DESCRIBING_CRTP_CHANNEL);
    assert!(!crtp_packet.payload.is_empty());

    // Verify the raw frame can be built
    let raw_frame = build_raw_crtp_frame(&crtp_packet.payload);
    assert!(raw_frame.len() > 3); // header + length + at least 1 byte payload + crc
}

#[test]
fn test_session_generates_streaming_ack() {
    let mut session = SelfDescribingSession::new();

    // Complete full handshake
    session.on_frame(&Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Test Device".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    }));

    session.on_frame(&Frame::CommandCatalogPage(CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![],
    }));

    session.on_frame(&Frame::VariableCatalogPage(VariableCatalogPage {
        page: 0,
        total_pages: 1,
        variables: vec![
            VariableDescriptor {
                name: "acc_x".to_string(),
                order: 0,
                unit: "m/s^2".to_string(),
                adjustable: false,
                value_type: ValueType::I16,
            },
            VariableDescriptor {
                name: "gain".to_string(),
                order: 1,
                unit: "".to_string(),
                adjustable: true,
                value_type: ValueType::F32,
            },
        ],
    }));

    // Host acknowledges variable catalog
    let responses = session.on_frame(&Frame::HostAck(HostAck {
        stage: AckStage::VariableCatalog,
        status: 0,
        message: "OK".to_string(),
    }));

    // Should have one response: Streaming ack
    assert_eq!(responses.len(), 1);
    match &responses[0] {
        Frame::HostAck(ack) => {
            assert_eq!(ack.stage, AckStage::Streaming);
            assert_eq!(ack.status, 0);
        }
        _ => panic!("expected Streaming ack response"),
    }

    assert!(session.is_streaming());
}

#[test]
fn test_set_variable_encode_decode_roundtrip() {
    let set_var = SetVariable {
        seq: 42,
        variable_index: 5,
        value: vec![0x00, 0x00, 0x80, 0x3F], // 1.0f32
    };

    let frame = Frame::SetVariable(set_var);
    let crtp_packet = encode_crtp_packet(&frame);

    // Verify the packet is identified as self-describing
    assert!(is_self_describing_packet(&crtp_packet));

    // Decode the packet
    let decoded = eadai::protocols::self_describing::decode_crtp_packet(&crtp_packet)
        .expect("decode should succeed")
        .expect("should be Some");

    match decoded {
        Frame::SetVariable(sv) => {
            assert_eq!(sv.seq, 42);
            assert_eq!(sv.variable_index, 5);
            assert_eq!(sv.value, vec![0x00, 0x00, 0x80, 0x3F]);
        }
        _ => panic!("expected SetVariable frame"),
    }
}

#[test]
fn test_ack_result_encode_decode_roundtrip() {
    let result = AckResult {
        seq: 42,
        code: 0,
        message: "success".to_string(),
    };

    let frame = Frame::AckResult(result);
    let crtp_packet = encode_crtp_packet(&frame);

    // Verify the packet is identified as self-describing
    assert!(is_self_describing_packet(&crtp_packet));

    // Decode the packet
    let decoded = eadai::protocols::self_describing::decode_crtp_packet(&crtp_packet)
        .expect("decode should succeed")
        .expect("should be Some");

    match decoded {
        Frame::AckResult(ar) => {
            assert_eq!(ar.seq, 42);
            assert_eq!(ar.code, 0);
            assert_eq!(ar.message, "success");
        }
        _ => panic!("expected AckResult frame"),
    }
}

#[test]
fn test_host_ack_encode_decode_roundtrip() {
    let ack = HostAck {
        stage: AckStage::VariableCatalog,
        status: 0,
        message: "OK".to_string(),
    };

    let frame = Frame::HostAck(ack);
    let crtp_packet = encode_crtp_packet(&frame);

    // Verify the packet is identified as self-describing
    assert!(is_self_describing_packet(&crtp_packet));

    // Decode the packet
    let decoded = eadai::protocols::self_describing::decode_crtp_packet(&crtp_packet)
        .expect("decode should succeed")
        .expect("should be Some");

    match decoded {
        Frame::HostAck(ha) => {
            assert_eq!(ha.stage, AckStage::VariableCatalog);
            assert_eq!(ha.status, 0);
            assert_eq!(ha.message, "OK");
        }
        _ => panic!("expected HostAck frame"),
    }
}

#[test]
fn test_raw_crtp_frame_preserves_self_describing_payload() {
    let identity = Identity {
        protocol_version: 1,
        device_name: "CRTP Test".to_string(),
        firmware_version: "2.0.0".to_string(),
        sample_rate_hz: 200,
        variable_count: 5,
        command_count: 3,
        sample_payload_len: 20,
    };

    let frame = Frame::Identity(identity);
    let crtp_packet = encode_crtp_packet(&frame);
    let raw_frame = build_raw_crtp_frame(&crtp_packet.payload);

    // Decode the raw frame using CRTP decoder
    let mut decoder = eadai::protocols::CrtpDecoder::new(4096);
    let packets = decoder.push(&raw_frame);

    assert_eq!(packets.len(), 1);
    let packet = &packets[0];
    assert!(is_self_describing_packet(packet));

    // Decode the self-describing frame
    let decoded = eadai::protocols::self_describing::decode_crtp_packet(packet)
        .expect("decode should succeed")
        .expect("should be Some");

    match decoded {
        Frame::Identity(id) => {
            assert_eq!(id.device_name, "CRTP Test");
            assert_eq!(id.sample_rate_hz, 200);
        }
        _ => panic!("expected identity frame"),
    }
}
