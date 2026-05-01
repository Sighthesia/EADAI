use eadai::protocols::crtp::{CrtpDecoder, CrtpPacket, CrtpPort};
use eadai::protocols::capability::{crtp_to_capabilities, CapabilityEvent};

#[test]
fn decodes_single_crtp_frame() {
    let frame = build_crtp_frame(0x00, 0x00, b"hello");
    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    let packet = &packets[0];
    assert_eq!(packet.port, CrtpPort::Console);
    assert_eq!(packet.channel, 0);
    assert_eq!(packet.payload, b"hello");
}

#[test]
fn decodes_multiple_frames() {
    let frame1 = build_crtp_frame(0x01, 0x00, &[0x10, 0x20]);
    let frame2 = build_crtp_frame(0x04, 0x01, &[0x30, 0x40, 0x50]);
    let mut combined = frame1;
    combined.extend_from_slice(&frame2);

    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&combined);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].port, CrtpPort::Parameter);
    assert_eq!(packets[1].port, CrtpPort::Logging);
    assert_eq!(packets[1].channel, 1);
}

#[test]
fn rejects_frame_with_bad_crc() {
    let mut frame = build_crtp_frame(0x00, 0x00, &[0x01]);
    let last = frame.len() - 1;
    frame[last] ^= 0xFF;

    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&frame);
    assert!(packets.is_empty());
}

#[test]
fn handles_invalid_length_gracefully() {
    // Create input with a length byte > 63 (invalid CRTP length).
    // Verify the decoder handles it without panicking or getting stuck.
    let mut frame = vec![0x00, 0x80]; // header=0x00, length=0x80 (>63)
    frame.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);

    let mut decoder = CrtpDecoder::new(4096);
    // Should not panic or hang
    let _packets = decoder.push(&frame);

    // Decoder should still work with valid frames afterward
    let valid_frame = build_crtp_frame(0x00, 0x00, b"test");
    let packets = decoder.push(&valid_frame);
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].payload, b"test");
}

#[test]
fn handles_partial_frame() {
    let frame = build_crtp_frame(0x00, 0x00, &[0x01, 0x02, 0x03]);
    let partial = &frame[..2];

    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(partial);
    assert!(packets.is_empty());
}

#[test]
fn packet_fields_map() {
    let packet = CrtpPacket {
        port: CrtpPort::Logging,
        channel: 1,
        payload: vec![0xAA, 0xBB],
    };

    let fields = packet.fields();
    assert_eq!(fields.get("port").unwrap(), "logging");
    assert_eq!(fields.get("channel").unwrap(), "1");
    assert_eq!(fields.get("payload_len").unwrap(), "2");
}

#[test]
fn console_port_includes_text_in_fields() {
    let packet = CrtpPacket {
        port: CrtpPort::Console,
        channel: 0,
        payload: b"test output".to_vec(),
    };

    let fields = packet.fields();
    assert_eq!(fields.get("text").unwrap(), "test output");
}

#[test]
fn decodes_all_port_types() {
    let ports = [
        (0x00, CrtpPort::Console),
        (0x01, CrtpPort::Parameter),
        (0x02, CrtpPort::Commander),
        (0x03, CrtpPort::Memory),
        (0x04, CrtpPort::Logging),
        (0x05, CrtpPort::HighLevelCommander),
        (0x06, CrtpPort::Setting),
        (0x07, CrtpPort::Debug),
    ];

    for (port_id, expected_port) in ports {
        let frame = build_crtp_frame(port_id, 0x00, &[0x01]);
        let mut decoder = CrtpDecoder::new(4096);
        let packets = decoder.push(&frame);
        assert_eq!(packets.len(), 1, "port_id={port_id}");
        assert_eq!(packets[0].port, expected_port, "port_id={port_id}");
    }
}

#[test]
fn decodes_empty_payload_frame() {
    let frame = build_crtp_frame(0x02, 0x00, &[]);
    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    assert!(packets[0].payload.is_empty());
}

#[test]
fn decoder_skips_garbage() {
    let garbage: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0xFC];
    let frame = build_crtp_frame(0x00, 0x00, b"test");
    let mut combined = garbage;
    combined.extend_from_slice(&frame);

    let mut decoder = CrtpDecoder::new(4096);
    let packets = decoder.push(&combined);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].payload, b"test");
}

fn build_crtp_frame(port: u8, channel: u8, payload: &[u8]) -> Vec<u8> {
    let header = ((port & 0x07) << 5) | (channel & 0x03);
    let mut frame = vec![header, payload.len() as u8];
    frame.extend_from_slice(payload);

    let mut crc: u8 = 0;
    for &byte in &frame {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x1D;
            } else {
                crc <<= 1;
            }
        }
    }
    frame.push(crc);
    frame
}

// --- New semantic mapping tests ---

#[test]
fn debug_port_includes_text_in_fields() {
    let packet = CrtpPacket {
        port: CrtpPort::Debug,
        channel: 0,
        payload: b"debug output line".to_vec(),
    };
    let fields = packet.fields();
    assert_eq!(fields.get("text").unwrap(), "debug output line");
}

#[test]
fn high_level_commander_go_to_with_yaw() {
    // go_to command with x, y, z, yaw: command(1) + 4*4 = 17 bytes
    let mut payload = vec![0u8; 17];
    payload[0] = 1; // go_to
    let x = 1.0f32;
    payload[1..5].copy_from_slice(&x.to_le_bytes());
    let y = 2.0f32;
    payload[5..9].copy_from_slice(&y.to_le_bytes());
    let z = 0.5f32;
    payload[9..13].copy_from_slice(&z.to_le_bytes());
    let yaw = std::f32::consts::FRAC_PI_2; // 1.5708 rad = 90°
    payload[13..17].copy_from_slice(&yaw.to_le_bytes());

    let packet = CrtpPacket {
        port: CrtpPort::HighLevelCommander,
        channel: 0,
        payload,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("command").unwrap(), "go_to");
    assert!(fields.get("yaw").unwrap().contains("90.0°"));
}

#[test]
fn high_level_commander_extra_commands() {
    // takeoff command
    let packet_takeoff = CrtpPacket {
        port: CrtpPort::HighLevelCommander,
        channel: 0,
        payload: vec![3], // takeoff
    };
    let fields = packet_takeoff.fields();
    assert_eq!(fields.get("command").unwrap(), "takeoff");

    // land command
    let packet_land = CrtpPacket {
        port: CrtpPort::HighLevelCommander,
        channel: 0,
        payload: vec![4], // land
    };
    let fields = packet_land.fields();
    assert_eq!(fields.get("command").unwrap(), "land");

    // hover command
    let packet_hover = CrtpPacket {
        port: CrtpPort::HighLevelCommander,
        channel: 0,
        payload: vec![6], // hover
    };
    let fields = packet_hover.fields();
    assert_eq!(fields.get("command").unwrap(), "hover");
}

#[test]
fn commander_emergency_stop_command() {
    let packet = CrtpPacket {
        port: CrtpPort::Commander,
        channel: 3,
        payload: vec![4], // emergency_stop
    };
    let fields = packet.fields();
    assert_eq!(fields.get("command").unwrap(), "emergency_stop");
}

#[test]
fn commander_rpyt_produces_attitude_capability() {
    let mut payload = vec![0u8; 16];
    // roll: 0.1 rad
    payload[0..4].copy_from_slice(&0.1f32.to_le_bytes());
    // pitch: -0.2 rad
    payload[4..8].copy_from_slice(&(-0.2f32).to_le_bytes());
    // yaw: 0.5 rad
    payload[8..12].copy_from_slice(&0.5f32.to_le_bytes());
    // thrust: 0.5
    payload[12..16].copy_from_slice(&0.5f32.to_le_bytes());

    let packet = CrtpPacket {
        port: CrtpPort::Commander,
        channel: 0,
        payload,
    };

    let events = crtp_to_capabilities(&packet);
    let attitude = events.iter().find_map(|e| match e {
        CapabilityEvent::Attitude(a) => Some(a),
        _ => None,
    });
    assert!(attitude.is_some(), "should produce Attitude capability");
    let a = attitude.unwrap();
    assert!((a.roll - 0.1).abs() < 0.01);
    assert!((a.pitch - (-0.2)).abs() < 0.01);
    assert!((a.yaw - 0.5).abs() < 0.01);
    assert_eq!(a.source_protocol, "crtp");

    // RawPacket should also be present
    let raw = events
        .iter()
        .find(|e| matches!(e, CapabilityEvent::RawPacket(_)));
    assert!(raw.is_some(), "should always emit RawPacket");
}

#[test]
fn non_commander_crtp_only_produces_raw_packet() {
    let packet = CrtpPacket {
        port: CrtpPort::Console,
        channel: 0,
        payload: b"hello".to_vec(),
    };
    let events = crtp_to_capabilities(&packet);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], CapabilityEvent::RawPacket(_)));
}
