use eadai::protocols::self_describing::{
    bitmap::BitmapCodec,
    codec::{decode_frame, encode_frame},
    frame::*,
    state::{HandshakeMachine, HandshakeState},
};

#[test]
fn test_identity_frame_roundtrip() {
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
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode identity");

    match decoded {
        Frame::Identity(d) => {
            assert_eq!(d.protocol_version, 1);
            assert_eq!(d.device_name, "Test Device");
            assert_eq!(d.firmware_version, "1.0.0");
            assert_eq!(d.sample_rate_hz, 100);
            assert_eq!(d.variable_count, 10);
            assert_eq!(d.command_count, 5);
            assert_eq!(d.sample_payload_len, 40);
        }
        _ => panic!("expected identity frame"),
    }
}

#[test]
fn test_variable_catalog_page_roundtrip() {
    let page = VariableCatalogPage {
        page: 0,
        total_pages: 2,
        variables: vec![
            VariableDescriptor {
                name: "acc_x".to_string(),
                order: 0,
                unit: "m/s^2".to_string(),
                adjustable: false,
                value_type: ValueType::I16,
            },
            VariableDescriptor {
                name: "pid_gain".to_string(),
                order: 1,
                unit: "".to_string(),
                adjustable: true,
                value_type: ValueType::F32,
            },
        ],
    };

    let frame = Frame::VariableCatalogPage(page);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode variable catalog page");

    match decoded {
        Frame::VariableCatalogPage(d) => {
            assert_eq!(d.page, 0);
            assert_eq!(d.total_pages, 2);
            assert_eq!(d.variables.len(), 2);
            assert_eq!(d.variables[0].name, "acc_x");
            assert_eq!(d.variables[0].value_type, ValueType::I16);
            assert!(!d.variables[0].adjustable);
            assert_eq!(d.variables[1].name, "pid_gain");
            assert!(d.variables[1].adjustable);
        }
        _ => panic!("expected variable catalog page frame"),
    }
}

#[test]
fn test_command_catalog_page_roundtrip() {
    let page = CommandCatalogPage {
        page: 0,
        total_pages: 1,
        commands: vec![CommandDescriptor {
            id: "start".to_string(),
            params: "none".to_string(),
            docs: "Start streaming".to_string(),
        }],
    };

    let frame = Frame::CommandCatalogPage(page);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode command catalog page");

    match decoded {
        Frame::CommandCatalogPage(d) => {
            assert_eq!(d.page, 0);
            assert_eq!(d.total_pages, 1);
            assert_eq!(d.commands.len(), 1);
            assert_eq!(d.commands[0].id, "start");
            assert_eq!(d.commands[0].docs, "Start streaming");
        }
        _ => panic!("expected command catalog page frame"),
    }
}

#[test]
fn test_host_ack_roundtrip() {
    let ack = HostAck {
        stage: AckStage::VariableCatalog,
        status: 0,
        message: "OK".to_string(),
    };

    let frame = Frame::HostAck(ack);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode host ack");

    match decoded {
        Frame::HostAck(d) => {
            assert_eq!(d.stage, AckStage::VariableCatalog);
            assert_eq!(d.status, 0);
            assert_eq!(d.message, "OK");
        }
        _ => panic!("expected host ack frame"),
    }
}

#[test]
fn test_telemetry_sample_roundtrip() {
    let sample = TelemetrySample {
        seq: 42,
        changed_bitmap: vec![0b10101010, 0b00000001],
        values: vec![1, 2, 3, 4],
    };

    let frame = Frame::TelemetrySample(sample);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode telemetry sample");

    match decoded {
        Frame::TelemetrySample(d) => {
            assert_eq!(d.seq, 42);
            assert_eq!(d.changed_bitmap, vec![0b10101010, 0b00000001]);
            assert_eq!(d.values, vec![1, 2, 3, 4]);
        }
        _ => panic!("expected telemetry sample frame"),
    }
}

#[test]
fn test_set_variable_roundtrip() {
    let set_var = SetVariable {
        seq: 10,
        variable_index: 5,
        value: vec![0x00, 0x01, 0x02, 0x03],
    };

    let frame = Frame::SetVariable(set_var);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode set variable");

    match decoded {
        Frame::SetVariable(d) => {
            assert_eq!(d.seq, 10);
            assert_eq!(d.variable_index, 5);
            assert_eq!(d.value, vec![0x00, 0x01, 0x02, 0x03]);
        }
        _ => panic!("expected set variable frame"),
    }
}

#[test]
fn test_ack_result_roundtrip() {
    let result = AckResult {
        seq: 10,
        code: 0,
        message: "success".to_string(),
    };

    let frame = Frame::AckResult(result);
    let encoded = encode_frame(&frame);
    let decoded = decode_frame(&encoded).expect("decode ack result");

    match decoded {
        Frame::AckResult(d) => {
            assert_eq!(d.seq, 10);
            assert_eq!(d.code, 0);
            assert_eq!(d.message, "success");
        }
        _ => panic!("expected ack result frame"),
    }
}

#[test]
fn test_handshake_complete_flow() {
    let mut machine = HandshakeMachine::new();
    assert_eq!(*machine.state(), HandshakeState::WaitingIdentity);

    // Receive identity
    machine
        .on_identity(Identity {
            protocol_version: 1,
            device_name: "Test Device".to_string(),
            firmware_version: "1.0.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 2,
            command_count: 1,
            sample_payload_len: 8,
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

    // Receive command catalog
    machine
        .on_command_catalog_page(CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "".to_string(),
                docs: "Start streaming".to_string(),
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    // Receive variable catalog
    machine
        .on_variable_catalog_page(VariableCatalogPage {
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
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

    // Host acknowledges
    machine
        .on_host_ack(&HostAck {
            stage: AckStage::VariableCatalog,
            status: 0,
            message: "OK".to_string(),
        })
        .unwrap();
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
fn test_bitmap_codec_compression() {
    let variables = vec![
        VariableDescriptor {
            name: "acc_x".to_string(),
            order: 0,
            unit: "m/s^2".to_string(),
            adjustable: false,
            value_type: ValueType::I16,
        },
        VariableDescriptor {
            name: "acc_y".to_string(),
            order: 1,
            unit: "m/s^2".to_string(),
            adjustable: false,
            value_type: ValueType::I16,
        },
        VariableDescriptor {
            name: "gain".to_string(),
            order: 2,
            unit: "".to_string(),
            adjustable: true,
            value_type: ValueType::F32,
        },
    ];

    let mut codec = BitmapCodec::new(variables);

    // First sample - all changed
    let mut values1 = Vec::new();
    values1.extend_from_slice(&100i16.to_le_bytes());
    values1.extend_from_slice(&200i16.to_le_bytes());
    values1.extend_from_slice(&1.5f32.to_le_bytes());
    let (sample1, unchanged1) = codec.encode(&values1, 1).unwrap();
    assert_eq!(unchanged1, 0);
    assert_eq!(sample1.changed_bitmap[0], 0b00000111);
    assert_eq!(sample1.values.len(), 8);

    // Second sample - only acc_x and gain changed
    let mut values2 = Vec::new();
    values2.extend_from_slice(&150i16.to_le_bytes());
    values2.extend_from_slice(&200i16.to_le_bytes());
    values2.extend_from_slice(&2.0f32.to_le_bytes());
    let (sample2, unchanged2) = codec.encode(&values2, 2).unwrap();
    assert_eq!(unchanged2, 1);
    assert_eq!(sample2.changed_bitmap[0], 0b00000101);
    assert_eq!(sample2.values.len(), 6);

    // Decode should reconstruct full values
    let decoded = codec.decode(&sample2).unwrap();
    assert_eq!(decoded.len(), 8);
    assert_eq!(i16::from_le_bytes([decoded[0], decoded[1]]), 150);
    assert_eq!(i16::from_le_bytes([decoded[2], decoded[3]]), 200);
    assert!(
        (f32::from_le_bytes([decoded[4], decoded[5], decoded[6], decoded[7]]) - 2.0).abs() < 0.001
    );
}

#[test]
fn test_bitmap_codec_no_changes() {
    let variables = vec![VariableDescriptor {
        name: "value".to_string(),
        order: 0,
        unit: "".to_string(),
        adjustable: false,
        value_type: ValueType::U32,
    }];

    let mut codec = BitmapCodec::new(variables);
    let values = vec![42u32.to_le_bytes()].concat();

    // First sample
    codec.encode(&values, 1).unwrap();

    // Second sample with same value
    let (sample, unchanged) = codec.encode(&values, 2).unwrap();
    assert_eq!(unchanged, 1);
    assert_eq!(sample.changed_bitmap[0], 0);
    assert!(sample.values.is_empty());

    // Decode should use previous value
    let decoded = codec.decode(&sample).unwrap();
    assert_eq!(
        u32::from_le_bytes([decoded[0], decoded[1], decoded[2], decoded[3]]),
        42
    );
}

#[test]
fn test_decode_error_truncated() {
    let data = [0x01]; // Identity frame type but no payload
    assert!(decode_frame(&data).is_err());
}

#[test]
fn test_decode_error_invalid_frame_type() {
    let data = [0xFF];
    assert!(matches!(
        decode_frame(&data),
        Err(eadai::protocols::self_describing::DecodeError::InvalidFrameType(0xFF))
    ));
}

#[test]
fn test_handshake_duplicate_page_error() {
    let mut machine = HandshakeMachine::new();
    machine
        .on_identity(Identity {
            protocol_version: 1,
            device_name: "Test".to_string(),
            firmware_version: "1.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 0,
            command_count: 1,
            sample_payload_len: 0,
        })
        .unwrap();

    machine
        .on_command_catalog_page(CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![],
        })
        .unwrap();

    // Duplicate page should fail
    assert!(
        machine
            .on_command_catalog_page(CommandCatalogPage {
                page: 0,
                total_pages: 1,
                commands: vec![],
            })
            .is_err()
    );
}

#[test]
fn test_handshake_wrong_state_error() {
    let mut machine = HandshakeMachine::new();
    // Try to send command catalog before identity
    assert!(
        machine
            .on_command_catalog_page(CommandCatalogPage {
                page: 0,
                total_pages: 1,
                commands: vec![],
            })
            .is_err()
    );
}

#[test]
fn test_handshake_with_intermediate_acks() {
    let mut machine = HandshakeMachine::new();

    // Receive identity
    machine
        .on_identity(Identity {
            protocol_version: 1,
            device_name: "Test".to_string(),
            firmware_version: "1.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 1,
            command_count: 1,
            sample_payload_len: 4,
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

    // Host sends identity ack (valid after identity received)
    machine
        .on_host_ack(&HostAck {
            stage: AckStage::Identity,
            status: 0,
            message: "OK".to_string(),
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

    // Receive command catalog
    machine
        .on_command_catalog_page(CommandCatalogPage {
            page: 0,
            total_pages: 1,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "".to_string(),
                docs: "Start".to_string(),
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    // Host sends command catalog ack (valid after catalog received)
    machine
        .on_host_ack(&HostAck {
            stage: AckStage::CommandCatalog,
            status: 0,
            message: "OK".to_string(),
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    // Receive variable catalog
    machine
        .on_variable_catalog_page(VariableCatalogPage {
            page: 0,
            total_pages: 1,
            variables: vec![VariableDescriptor {
                name: "val".to_string(),
                order: 0,
                unit: "".to_string(),
                adjustable: false,
                value_type: ValueType::U32,
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

    // Host acknowledges variable catalog
    machine
        .on_host_ack(&HostAck {
            stage: AckStage::VariableCatalog,
            status: 0,
            message: "OK".to_string(),
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::ReadyToStream);

    // Start streaming
    machine.start_streaming().unwrap();
    assert_eq!(*machine.state(), HandshakeState::Streaming);
}

#[test]
fn test_handshake_catalog_sorted_by_page_order() {
    let mut machine = HandshakeMachine::new();
    machine
        .on_identity(Identity {
            protocol_version: 1,
            device_name: "Test".to_string(),
            firmware_version: "1.0".to_string(),
            sample_rate_hz: 100,
            variable_count: 3,
            command_count: 2,
            sample_payload_len: 12,
        })
        .unwrap();

    // Send pages out of order
    machine
        .on_command_catalog_page(CommandCatalogPage {
            page: 1,
            total_pages: 2,
            commands: vec![CommandDescriptor {
                id: "stop".to_string(),
                params: "".to_string(),
                docs: "Stop".to_string(),
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingCommandCatalog);

    machine
        .on_command_catalog_page(CommandCatalogPage {
            page: 0,
            total_pages: 2,
            commands: vec![CommandDescriptor {
                id: "start".to_string(),
                params: "".to_string(),
                docs: "Start".to_string(),
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    // Commands should be sorted by page order
    let catalog = machine.command_catalog();
    assert_eq!(catalog.len(), 2);
    assert_eq!(catalog[0].id, "start");
    assert_eq!(catalog[1].id, "stop");

    // Send variable pages out of order
    machine
        .on_variable_catalog_page(VariableCatalogPage {
            page: 1,
            total_pages: 2,
            variables: vec![VariableDescriptor {
                name: "gain".to_string(),
                order: 2,
                unit: "".to_string(),
                adjustable: true,
                value_type: ValueType::F32,
            }],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingVariableCatalog);

    machine
        .on_variable_catalog_page(VariableCatalogPage {
            page: 0,
            total_pages: 2,
            variables: vec![
                VariableDescriptor {
                    name: "acc_x".to_string(),
                    order: 0,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
                VariableDescriptor {
                    name: "acc_y".to_string(),
                    order: 1,
                    unit: "m/s^2".to_string(),
                    adjustable: false,
                    value_type: ValueType::I16,
                },
            ],
        })
        .unwrap();
    assert_eq!(*machine.state(), HandshakeState::WaitingHostAck);

    // Variables should be sorted by order field
    let var_catalog = machine.variable_catalog();
    assert_eq!(var_catalog.len(), 3);
    assert_eq!(var_catalog[0].name, "acc_x");
    assert_eq!(var_catalog[1].name, "acc_y");
    assert_eq!(var_catalog[2].name, "gain");
}
