use eadai::protocols::self_describing::{
    crtp_adapter::{
        RawSelfDescribingDecoder, SELF_DESCRIBING_CRTP_CHANNEL, SELF_DESCRIBING_CRTP_PORT,
        encode_crtp_packet, encode_raw_transport_frame, is_self_describing_packet,
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

fn build_raw_self_describing_frame(frame: &Frame) -> Vec<u8> {
    encode_raw_transport_frame(frame)
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
fn test_session_generates_staged_host_acks() {
    let mut session = SelfDescribingSession::new();

    // Identity should trigger the first staged ack.
    let responses = session.on_frame(&Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Test Device".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    }));
    assert_eq!(responses.len(), 1);
    assert!(matches!(responses[0], Frame::HostAck(HostAck { stage: AckStage::Identity })));

    // Command catalog completion should trigger the second staged ack.
    let responses = session.on_frame(&Frame::CommandCatalogPage(CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![],
    }));
    assert_eq!(responses.len(), 1);
    assert!(matches!(responses[0], Frame::HostAck(HostAck { stage: AckStage::CommandCatalog })));

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
    }));

    // Should have one response: Streaming ack
    assert_eq!(responses.len(), 1);
    match &responses[0] {
        Frame::HostAck(ack) => {
            assert_eq!(ack.stage, AckStage::Streaming);
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
        }
        _ => panic!("expected HostAck frame"),
    }
}

#[test]
fn test_raw_host_ack_frame_payload_is_compact() {
    let raw = build_raw_self_describing_frame(&Frame::HostAck(HostAck {
        stage: AckStage::Identity,
    }));

    assert_eq!(raw[0], 0x73);
    assert_eq!(raw[1], 2);
    assert_eq!(&raw[2..], &[0x04, 0x01]);
    assert_eq!(raw.len(), 4);
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

#[test]
fn test_raw_self_describing_decoder_handles_identity_and_ack_sequence() {
    let mut decoder = RawSelfDescribingDecoder::new(4096);

    let identity = Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Raw Test".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    });

    let frames = decoder.push(&build_raw_self_describing_frame(&identity));
    assert_eq!(frames.len(), 1);
    assert!(matches!(frames[0], Frame::Identity(_)));

    let command_catalog = Frame::CommandCatalogPage(CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![],
    });
    let frames = decoder.push(&build_raw_self_describing_frame(&command_catalog));
    assert_eq!(frames.len(), 1);
    assert!(matches!(frames[0], Frame::CommandCatalogPage(_)));

    let variable_catalog = Frame::VariableCatalogPage(VariableCatalogPage {
        page: 0,
        total_pages: 1,
        variables: vec![],
    });
    let frames = decoder.push(&build_raw_self_describing_frame(&variable_catalog));
    assert_eq!(frames.len(), 1);
    assert!(matches!(frames[0], Frame::VariableCatalogPage(_)));
}

#[test]
fn test_raw_self_describing_decoder_skips_crtp_empty_packet_false_positive() {
    let mut decoder = RawSelfDescribingDecoder::new(4096);
    let identity = Frame::Identity(Identity {
        protocol_version: 1,
        device_name: "Raw Test".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    });

    let frames = decoder.push(&build_raw_self_describing_frame(&identity));
    assert_eq!(frames.len(), 1);

    // Feed the same canonical raw bytes again to ensure the decoder stays aligned
    // and does not create a fake CRTP console/channel-0 empty payload packet.
    let frames = decoder.push(&build_raw_self_describing_frame(&identity));
    assert_eq!(frames.len(), 1);
}
