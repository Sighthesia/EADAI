use eadai::protocols::crtp::{CrtpDecoder, CrtpPacket, CrtpPort};
use eadai::protocols::self_describing::{
    crtp_adapter::{
        SELF_DESCRIBING_CRTP_CHANNEL, SELF_DESCRIBING_CRTP_PORT, decode_crtp_packet,
        encode_crtp_packet, is_self_describing_packet,
    },
    frame::*,
    state::{HandshakeMachine, HandshakeState},
};

/// Helper to build a CRTP frame for the self-describing protocol.
fn build_self_describing_crtp_frame(payload: &[u8]) -> Vec<u8> {
    let header = ((SELF_DESCRIBING_CRTP_PORT & 0x07) << 5) | (SELF_DESCRIBING_CRTP_CHANNEL & 0x03);
    let mut frame = vec![header, payload.len() as u8];
    frame.extend_from_slice(payload);
    // CRC-8 calculation (same as CRTP)
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
fn test_crtp_adapter_identifies_self_describing_packets() {
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
fn test_crtp_adapter_decodes_identity_frame() {
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
    let crtp_payload = eadai::protocols::self_describing::encode_frame(&frame);
    let packet = CrtpPacket {
        port: CrtpPort::Debug,
        channel: SELF_DESCRIBING_CRTP_CHANNEL,
        payload: crtp_payload,
    };

    let decoded = decode_crtp_packet(&packet).expect("decode should succeed");
    match decoded {
        Some(Frame::Identity(d)) => {
            assert_eq!(d.device_name, "Test Device");
            assert_eq!(d.sample_rate_hz, 100);
            assert_eq!(d.variable_count, 10);
        }
        _ => panic!("expected identity frame"),
    }
}

#[test]
fn test_crtp_adapter_encodes_and_decodes_roundtrip() {
    let sample = TelemetrySample {
        seq: 42,
        changed_bitmap: vec![0b10101010, 0b00000001],
        values: vec![1, 2, 3, 4],
    };

    let frame = Frame::TelemetrySample(sample);
    let crtp_packet = encode_crtp_packet(&frame);

    assert_eq!(crtp_packet.port, CrtpPort::Debug);
    assert_eq!(crtp_packet.channel, SELF_DESCRIBING_CRTP_CHANNEL);

    let decoded = decode_crtp_packet(&crtp_packet).expect("decode should succeed");
    match decoded {
        Some(Frame::TelemetrySample(d)) => {
            assert_eq!(d.seq, 42);
            assert_eq!(d.changed_bitmap, vec![0b10101010, 0b00000001]);
            assert_eq!(d.values, vec![1, 2, 3, 4]);
        }
        _ => panic!("expected telemetry sample frame"),
    }
}

#[test]
fn test_crtp_decoder_can_decode_self_describing_frames() {
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
    let crtp_payload = eadai::protocols::self_describing::encode_frame(&frame);
    let raw_frame = build_self_describing_crtp_frame(&crtp_payload);

    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&raw_frame);

    assert_eq!(packets.len(), 1);
    let packet = &packets[0];
    assert!(is_self_describing_packet(packet));

    let decoded = decode_crtp_packet(packet).expect("decode should succeed");
    match decoded {
        Some(Frame::Identity(d)) => {
            assert_eq!(d.device_name, "CRTP Test");
            assert_eq!(d.sample_rate_hz, 200);
        }
        _ => panic!("expected identity frame"),
    }
}

#[test]
fn test_handshake_flow_over_crtp() {
    let mut machine = HandshakeMachine::new();
    assert_eq!(*machine.state(), HandshakeState::WaitingIdentity);

    // Simulate receiving identity over CRTP
    let identity = Identity {
        protocol_version: 1,
        device_name: "CRTP Device".to_string(),
        firmware_version: "1.0.0".to_string(),
        sample_rate_hz: 100,
        variable_count: 2,
        command_count: 1,
        sample_payload_len: 8,
    };

    let frame = Frame::Identity(identity);
    let crtp_packet = encode_crtp_packet(&frame);
    let decoded = decode_crtp_packet(&crtp_packet).unwrap().unwrap();

    match decoded {
        Frame::Identity(id) => machine.on_identity(id).unwrap(),
        _ => panic!("expected identity"),
    }
    assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

    // Simulate receiving command catalog over CRTP
    let cmd_catalog = CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![CommandDescriptor {
            id: "start".to_string(),
            params: "".to_string(),
            docs: "Start streaming".to_string(),
        }],
    };

    let frame = Frame::CommandCatalogPage(cmd_catalog);
    let crtp_packet = encode_crtp_packet(&frame);
    let decoded = decode_crtp_packet(&crtp_packet).unwrap().unwrap();

    match decoded {
        Frame::CommandCatalogPage(cat) => machine.on_command_catalog_page(cat).unwrap(),
        _ => panic!("expected command catalog"),
    }
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    // Simulate receiving variable catalog over CRTP
    let var_catalog = VariableCatalogPage {
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
    };

    let frame = Frame::VariableCatalogPage(var_catalog);
    let crtp_packet = encode_crtp_packet(&frame);
    let decoded = decode_crtp_packet(&crtp_packet).unwrap().unwrap();

    match decoded {
        Frame::VariableCatalogPage(cat) => machine.on_variable_catalog_page(cat).unwrap(),
        _ => panic!("expected variable catalog"),
    }
    assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

    // Simulate host acknowledgment
    let ack = HostAck {
        stage: AckStage::VariableCatalog,
    };

    machine.on_host_ack(&ack).unwrap();
    assert_eq!(*machine.state(), HandshakeState::ReadyToStream);

    // Start streaming
    machine.start_streaming().unwrap();
    assert_eq!(*machine.state(), HandshakeState::Streaming);

    // Verify catalogs
    let cmd_catalog = machine.command_catalog();
    assert_eq!(cmd_catalog.len(), 1);
    assert_eq!(cmd_catalog[0].id, "start");

    let var_catalog = machine.variable_catalog();
    assert_eq!(var_catalog.len(), 2);
    assert_eq!(var_catalog[0].name, "acc_x");
    assert_eq!(var_catalog[1].name, "gain");
}

#[test]
fn test_set_variable_over_crtp() {
    let set_var = SetVariable {
        seq: 10,
        variable_index: 1,
        value: 0x3F800000u32.to_le_bytes().to_vec(), // 1.0f32
    };

    let frame = Frame::SetVariable(set_var);
    let crtp_packet = encode_crtp_packet(&frame);
    let decoded = decode_crtp_packet(&crtp_packet).unwrap().unwrap();

    match decoded {
        Frame::SetVariable(d) => {
            assert_eq!(d.seq, 10);
            assert_eq!(d.variable_index, 1);
            assert_eq!(d.value, vec![0x00, 0x00, 0x80, 0x3F]);
        }
        _ => panic!("expected set variable frame"),
    }
}

#[test]
fn test_ack_result_over_crtp() {
    let result = AckResult {
        seq: 10,
        code: 0,
        message: "success".to_string(),
    };

    let frame = Frame::AckResult(result);
    let crtp_packet = encode_crtp_packet(&frame);
    let decoded = decode_crtp_packet(&crtp_packet).unwrap().unwrap();

    match decoded {
        Frame::AckResult(d) => {
            assert_eq!(d.seq, 10);
            assert_eq!(d.code, 0);
            assert_eq!(d.message, "success");
        }
        _ => panic!("expected ack result frame"),
    }
}
