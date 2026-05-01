use eadai::protocols::mavlink::{MavlinkDecoder, MavlinkPacket};

#[test]
fn decodes_single_v2_frame() {
    // HEARTBEAT (0x0000) has crc_extra=50
    let frame = build_mavlink_v2_frame(0x0000, &[0x01, 0x02, 0x03], 50);
    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    let packet = &packets[0];
    assert_eq!(packet.message_id, 0x0000);
    assert_eq!(packet.system_id, 1);
    assert_eq!(packet.component_id, 1);
    assert_eq!(packet.payload, vec![0x01, 0x02, 0x03]);
    assert!(packet.crc_validated);
}

#[test]
fn decodes_multiple_frames_in_chunk() {
    let frame1 = build_mavlink_v2_frame(0x0000, &[0x01], 50);
    let frame2 = build_mavlink_v2_frame(0x0001, &[0x02, 0x03], 124);
    let mut combined = frame1;
    combined.extend_from_slice(&frame2);

    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&combined);

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0].message_id, 0x0000);
    assert_eq!(packets[1].message_id, 0x0001);
}

#[test]
fn rejects_frame_with_bad_crc() {
    let mut frame = build_mavlink_v2_frame(0x0000, &[0x01], 50);
    let last = frame.len() - 1;
    frame[last] ^= 0xFF; // corrupt CRC

    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&frame);
    assert!(packets.is_empty());
}

#[test]
fn handles_partial_frame_gracefully() {
    let frame = build_mavlink_v2_frame(0x0000, &[0x01, 0x02], 50);
    let partial = &frame[..5];

    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(partial);
    assert!(packets.is_empty());
}

#[test]
fn decodes_empty_payload_frame() {
    let frame = build_mavlink_v2_frame(0x0003, &[], 104);
    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].message_id, 0x0003);
    assert!(packets[0].payload.is_empty());
}

#[test]
fn decodes_large_payload_frame() {
    let payload: Vec<u8> = (0..50).map(|i| (i % 256) as u8).collect();
    // Use SYS_STATUS (0x0001) which has crc_extra=124 in our table
    let frame = build_mavlink_v2_frame(0x0001, &payload, 124);
    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].payload.len(), 50);
    assert_eq!(packets[0].payload, payload);
}

#[test]
fn packet_fields_map() {
    let packet = MavlinkPacket {
        sequence: 42,
        system_id: 255,
        component_id: 1,
        message_id: 0x0245,
        payload: vec![],
        target_system: Some(1),
        target_component: Some(0),
        crc_validated: true,
    };

    let fields = packet.fields();
    assert_eq!(fields.get("message_id").unwrap(), "0x0245");
    assert_eq!(fields.get("system_id").unwrap(), "255");
    assert_eq!(fields.get("target_system").unwrap(), "1");
}

#[test]
fn decoder_skips_garbage_before_sof() {
    let garbage: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let frame = build_mavlink_v2_frame(0x0000, &[0xAA], 50);
    let mut combined = garbage;
    combined.extend_from_slice(&frame);

    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&combined);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].message_id, 0x0000);
}

#[test]
fn accepts_unknown_message_without_crc_table() {
    // Unknown message ID - should be accepted but marked as unvalidated
    let frame = build_mavlink_v2_frame(0xFFFF, &[0x01, 0x02], 0);
    let mut decoder = MavlinkDecoder::new(4096);
    let packets = decoder.push(&frame);

    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].message_id, 0xFFFF);
    assert!(packets[0].crc_validated);
}

fn build_mavlink_v2_frame(message_id: u32, payload: &[u8], crc_extra: u8) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(0xFD); // SOF
    frame.push(payload.len() as u8);
    frame.push(0); // incompat_flags
    frame.push(0); // compat_flags
    frame.push(1); // sequence
    frame.push(1); // system_id
    frame.push(1); // component_id
    let mid_bytes = message_id.to_le_bytes();
    frame.push(mid_bytes[0]);
    frame.push(mid_bytes[1]);
    frame.push(mid_bytes[2]);
    frame.extend_from_slice(payload);

    let mut crc_state: u16 = 0xFFFF;
    for &byte in &frame {
        crc_state ^= byte as u16;
        for _ in 0..8 {
            if crc_state & 1 != 0 {
                crc_state = (crc_state >> 1) ^ 0x8408;
            } else {
                crc_state >>= 1;
            }
        }
    }
    crc_state ^= crc_extra as u16;
    for _ in 0..8 {
        if crc_state & 1 != 0 {
            crc_state = (crc_state >> 1) ^ 0x8408;
        } else {
            crc_state >>= 1;
        }
    }

    frame.extend_from_slice(&crc_state.to_le_bytes());
    frame
}

// --- New semantic mapping tests for extended MAVLink messages ---

#[test]
fn system_time_semantic_fields() {
    // SYSTEM_TIME (0x0002) payload: time_unix_usec(8), time_boot_ms(4)
    let mut payload = vec![0u8; 12];
    // time_unix_usec: 1700000000000000 (0x0603689C8B4300)
    let t = 1_700_000_000_000_000u64;
    payload[0..8].copy_from_slice(&t.to_le_bytes());
    // time_boot_ms: 42000
    let boot = 42000u32;
    payload[8..12].copy_from_slice(&boot.to_le_bytes());

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x0002, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("time_boot_ms").unwrap(), "42000 ms");
    assert!(fields.get("time_unix_usec").is_some());
}

#[test]
fn attitude_quaternion_semantic_fields() {
    // ATTITUDE_QUATERNION (0x0033): time_usec(8), q1..q4(4 each), rollspeed, pitchspeed, yawspeed(4 each)
    let mut payload = vec![0u8; 36];
    let q1 = 1.0f32;
    let q2 = 0.0f32;
    let q3 = 0.0f32;
    let q4 = 0.0f32;
    payload[8..12].copy_from_slice(&q1.to_le_bytes());
    payload[12..16].copy_from_slice(&q2.to_le_bytes());
    payload[16..20].copy_from_slice(&q3.to_le_bytes());
    payload[20..24].copy_from_slice(&q4.to_le_bytes());
    let roll = 0.1f32;
    payload[24..28].copy_from_slice(&roll.to_le_bytes());

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x0033, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("q1").unwrap(), "1.0000");
    assert_eq!(fields.get("q2").unwrap(), "0.0000");
    assert!(fields.get("rollspeed").unwrap().contains("0.1"));
}

#[test]
fn local_position_ned_semantic_fields() {
    // LOCAL_POSITION_NED (0x0035): time_boot_ms(4), x(4), y(4), z(4), vx(4), vy(4), vz(4)
    let mut payload = vec![0u8; 28];
    let x = 1.5f32;
    payload[4..8].copy_from_slice(&x.to_le_bytes());
    let z = -2.0f32;
    payload[12..16].copy_from_slice(&z.to_le_bytes());

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x0035, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert!(fields.get("x").unwrap().contains("1.5"));
    assert!(fields.get("z").unwrap().contains("-2"));
}

#[test]
fn command_ack_semantic_fields() {
    // COMMAND_ACK (0x0053): command(2), result(1)
    let payload = vec![0x00, 0x01, 0x00]; // command=0x0100, result=0 (Accepted)
    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x0053, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("command").unwrap(), "0x0100");
    assert_eq!(fields.get("result").unwrap(), "Accepted");
}

#[test]
fn rc_channels_semantic_fields() {
    // RC_CHANNELS (0x00A0): time_boot_ms(4), ch1..ch18(2 each), rssi(1) = 41 bytes
    let mut payload = vec![0u8; 43];
    // ch1: 1500 us
    let ch1 = 1500u16;
    payload[4..6].copy_from_slice(&ch1.to_le_bytes());
    // rssi: 75%
    payload[42] = 75;

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x00A0, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("ch1").unwrap(), "1500 us");
    assert_eq!(fields.get("rssi").unwrap(), "75%");
}

#[test]
fn gps_status_semantic_fields() {
    // GPS_STATUS (0x00C7): satellites_visible[10]
    let payload = vec![12, 8, 5, 3, 0, 0, 0, 0, 0, 0];
    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x00C7, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("satellites_visible").unwrap(), "12, 8, 5, 3, 0, 0, 0, 0, 0, 0");
    assert_eq!(fields.get("satellite_count").unwrap(), "10");
}

#[test]
fn scaled_pressure_semantic_fields() {
    // SCALED_PRESSURE (0x00C9): time_boot_ms(4), press_abs(4), press_diff(4), temperature(2)
    let mut payload = vec![0u8; 14];
    let press = 1013.25f32;
    payload[4..8].copy_from_slice(&press.to_le_bytes());
    let temp: i16 = 2500; // 25.00 °C
    payload[12..14].copy_from_slice(&temp.to_le_bytes());

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x00C9, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert!(fields.get("press_abs").unwrap().contains("1013"));
    assert_eq!(fields.get("temperature").unwrap(), "25 °C");
}

#[test]
fn battery_status_semantic_fields() {
    // BATTERY_STATUS (0x00D0): battery_function(1), battery_type(1), temperature(2),
    // voltages[10](20), current_battery(2), current_consumed(4), energy_consumed(4), battery_remaining(1) = 35 bytes
    let mut payload = vec![0u8; 35];
    payload[0] = 1; // battery_function: All
    payload[1] = 1; // battery_type: LiPo
    let temp: i16 = 2500; // 25.00 °C
    payload[2..4].copy_from_slice(&temp.to_le_bytes());
    let voltage: u16 = 12600; // 12600 mV (first element of voltages array)
    payload[4..6].copy_from_slice(&voltage.to_le_bytes());
    let current: i16 = -1200; // -1200 cA (offset 24)
    payload[24..26].copy_from_slice(&current.to_le_bytes());
    payload[34] = 85; // 85% remaining (offset 34)

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x00D0, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert_eq!(fields.get("battery_function").unwrap(), "All");
    assert_eq!(fields.get("battery_type").unwrap(), "LiPo");
    assert_eq!(fields.get("temperature").unwrap(), "25 °C");
    assert_eq!(fields.get("voltage").unwrap(), "12600 mV");
    assert_eq!(fields.get("current").unwrap(), "-1200 mA");
    assert_eq!(fields.get("remaining").unwrap(), "85%");
}

#[test]
fn vibration_semantic_fields() {
    // VIBRATION (0x00FE): time_usec(8), vib_x(4), vib_y(4), vib_z(4), clip_0(4), clip_1(4), clip_2(4)
    let mut payload = vec![0u8; 32];
    let vx = 2.5f32;
    payload[8..12].copy_from_slice(&vx.to_le_bytes());
    let vy = 1.2f32;
    payload[12..16].copy_from_slice(&vy.to_le_bytes());
    let vz = 3.1f32;
    payload[16..20].copy_from_slice(&vz.to_le_bytes());

    let packet = MavlinkPacket {
        sequence: 1, system_id: 1, component_id: 1,
        message_id: 0x00FE, payload, target_system: None, target_component: None,
        crc_validated: true,
    };
    let fields = packet.fields();
    assert!(fields.get("vibration_x").unwrap().contains("2.5"));
    assert!(fields.get("vibration_y").unwrap().contains("1.2"));
    assert!(fields.get("vibration_z").unwrap().contains("3.1"));
}
