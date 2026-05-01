use std::collections::BTreeMap;

/// CRTP-over-serial framing constants.
///
/// CRTP-over-serial uses: [header][length][data...][crc8]
/// - header: 1 byte, bits 7-5 = port, bits 4-3 = reserved, bits 2-0 = channel
/// - length: 1 byte, payload length (0-63)
/// - data: payload bytes
/// - crc8: 1 byte, CRC-8 over header + length + data
const CRTP_HEADER_LEN: usize = 1;
const CRTP_LENGTH_LEN: usize = 1;
const CRTP_CRC_LEN: usize = 1;
const CRTP_MIN_FRAME_LEN: usize = CRTP_HEADER_LEN + CRTP_LENGTH_LEN + CRTP_CRC_LEN;

/// CRTP port identifiers for common Crazyflie subsystems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrtpPort {
    /// 0x0 - Console / logging
    Console,
    /// 0x1 - Parameters
    Parameter,
    /// 0x2 - Commander
    Commander,
    /// 0x3 - Memory access
    Memory,
    /// 0x4 - Logging
    Logging,
    /// 0x5 - High-level commander
    HighLevelCommander,
    /// 0x6 - Setting
    Setting,
    /// 0x7 - Debug data
    Debug,
    /// 0xD - Link layer
    Link,
    /// 0xF - Broadcasting
    Broadcasting,
    /// Unknown port
    Unknown(u8),
}

impl CrtpPort {
    fn from_header(header: u8) -> Self {
        match (header >> 5) & 0x07 {
            0x0 => Self::Console,
            0x1 => Self::Parameter,
            0x2 => Self::Commander,
            0x3 => Self::Memory,
            0x4 => Self::Logging,
            0x5 => Self::HighLevelCommander,
            0x6 => Self::Setting,
            0x7 => Self::Debug,
            _ => Self::Unknown((header >> 5) & 0x07),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Console => "console",
            Self::Parameter => "parameter",
            Self::Commander => "commander",
            Self::Memory => "memory",
            Self::Logging => "logging",
            Self::HighLevelCommander => "high_level_commander",
            Self::Setting => "setting",
            Self::Debug => "debug",
            Self::Link => "link",
            Self::Broadcasting => "broadcasting",
            Self::Unknown(_) => "unknown",
        }
    }
}

/// One decoded CRTP-over-serial packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrtpPacket {
    pub port: CrtpPort,
    pub channel: u8,
    pub payload: Vec<u8>,
}

impl CrtpPacket {
    pub fn fields(&self) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        fields.insert("port".into(), self.port.label().to_string());
        fields.insert("channel".into(), self.channel.to_string());
        fields.insert("payload_len".into(), self.payload.len().to_string());

        match &self.port {
            CrtpPort::Console => {
                let text = String::from_utf8_lossy(&self.payload);
                fields.insert("text".into(), text.into_owned());
            }
            CrtpPort::Parameter => {
                self.extract_parameter_fields(&mut fields);
            }
            CrtpPort::Commander => {
                self.extract_commander_fields(&mut fields);
            }
            CrtpPort::Memory => {
                self.extract_memory_fields(&mut fields);
            }
            CrtpPort::Logging => {
                self.extract_logging_fields(&mut fields);
            }
            CrtpPort::HighLevelCommander => {
                self.extract_high_level_commander_fields(&mut fields);
            }
            CrtpPort::Setting => {
                self.extract_setting_fields(&mut fields);
            }
            CrtpPort::Debug => {
                let text = String::from_utf8_lossy(&self.payload);
                fields.insert("text".into(), text.into_owned());
            }
            _ => {}
        }

        fields
    }

    /// Extract semantic fields from Parameter port packets.
    /// Channel 0: Read request/response
    /// Channel 1: Write request/response
    /// Channel 2: Toc info
    fn extract_parameter_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.is_empty() {
            return;
        }

        let first_byte = self.payload[0];
        match self.channel {
            0 => {
                // Read request/response
                fields.insert("operation".into(), "read".into());
                if self.payload.len() >= 2 {
                    let id = u16::from_le_bytes([self.payload[0], self.payload[1]]);
                    fields.insert("param_id".into(), id.to_string());
                }
            }
            1 => {
                // Write request/response
                fields.insert("operation".into(), "write".into());
                if self.payload.len() >= 2 {
                    let id = u16::from_le_bytes([self.payload[0], self.payload[1]]);
                    fields.insert("param_id".into(), id.to_string());
                    if self.payload.len() > 2 {
                        let value = match self.payload.len() {
                            3 => format!("{} (int8)", self.payload[2] as i8),
                            4 => format!(
                                "{} (int16)",
                                i16::from_le_bytes([self.payload[2], self.payload[3]])
                            ),
                            6 => {
                                let val = i32::from_le_bytes([
                                    self.payload[2],
                                    self.payload[3],
                                    self.payload[4],
                                    self.payload[5],
                                ]);
                                format!("{} (int32)", val)
                            }
                            _ => "unknown".into(),
                        };
                        fields.insert("param_value".into(), value);
                    }
                }
            }
            2 => {
                // TOC info
                fields.insert("operation".into(), "toc_info".into());
                if !self.payload.is_empty() {
                    fields.insert("toc_cmd".into(), format!("0x{:02X}", first_byte));
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }

    /// Extract semantic fields from Commander port packets.
    /// Channel 0: Roll/Pitch/Yaw/Thrust (RPYT)
    /// Channel 1: Altitude hold mode
    /// Channel 2: Velocity control
    /// Channel 3: High-level commands
    fn extract_commander_fields(&self, fields: &mut BTreeMap<String, String>) {
        match self.channel {
            0 => {
                // Roll/Pitch/Yaw/Thrust (RPYT) - 4 bytes each, float32
                fields.insert("control_mode".into(), "rpyt".into());
                if self.payload.len() >= 16 {
                    let roll = f32::from_le_bytes([
                        self.payload[0],
                        self.payload[1],
                        self.payload[2],
                        self.payload[3],
                    ]);
                    let pitch = f32::from_le_bytes([
                        self.payload[4],
                        self.payload[5],
                        self.payload[6],
                        self.payload[7],
                    ]);
                    let yaw = f32::from_le_bytes([
                        self.payload[8],
                        self.payload[9],
                        self.payload[10],
                        self.payload[11],
                    ]);
                    let thrust = f32::from_le_bytes([
                        self.payload[12],
                        self.payload[13],
                        self.payload[14],
                        self.payload[15],
                    ]);
                    fields.insert("roll".into(), format!("{:.3}", roll));
                    fields.insert("pitch".into(), format!("{:.3}", pitch));
                    fields.insert("yaw".into(), format!("{:.3}", yaw));
                    fields.insert("thrust".into(), format!("{:.3}", thrust));
                }
            }
            1 => {
                // Altitude hold mode
                fields.insert("control_mode".into(), "alt_hold".into());
                if self.payload.len() >= 4 {
                    let height = f32::from_le_bytes([
                        self.payload[0],
                        self.payload[1],
                        self.payload[2],
                        self.payload[3],
                    ]);
                    fields.insert("height".into(), format!("{:.2} m", height));
                }
            }
            2 => {
                // Velocity control (vx, vy, yaw_rate)
                fields.insert("control_mode".into(), "velocity".into());
                if self.payload.len() >= 12 {
                    let vx = f32::from_le_bytes([
                        self.payload[0],
                        self.payload[1],
                        self.payload[2],
                        self.payload[3],
                    ]);
                    let vy = f32::from_le_bytes([
                        self.payload[4],
                        self.payload[5],
                        self.payload[6],
                        self.payload[7],
                    ]);
                    let yaw_rate = f32::from_le_bytes([
                        self.payload[8],
                        self.payload[9],
                        self.payload[10],
                        self.payload[11],
                    ]);
                    fields.insert("vx".into(), format!("{} m/s", vx));
                    fields.insert("vy".into(), format!("{} m/s", vy));
                    fields.insert("yaw_rate".into(), format!("{} rad/s", yaw_rate));
                }
            }
            3 => {
                // High-level commands
                fields.insert("control_mode".into(), "high_level".into());
                if !self.payload.is_empty() {
                    let cmd = self.payload[0];
                    fields.insert("command".into(), commander_command_label(cmd).to_string());
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }

    /// Extract semantic fields from Memory port packets.
    /// Channel 0: Read/Write requests
    /// Channel 1: Read/Write responses
    fn extract_memory_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.is_empty() {
            return;
        }

        match self.channel {
            0 => {
                // Request
                fields.insert("operation".into(), "request".into());
                if self.payload.len() >= 2 {
                    let cmd = self.payload[1];
                    fields.insert("memory_cmd".into(), memory_command_label(cmd).to_string());
                }
            }
            1 => {
                // Response
                fields.insert("operation".into(), "response".into());
                if self.payload.len() >= 2 {
                    let status = self.payload[1];
                    fields.insert("status".into(), memory_status_label(status).to_string());
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }

    /// Extract semantic fields from Logging port packets.
    /// Channel 0: Control (start/stop)
    /// Channel 1: Data
    fn extract_logging_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.is_empty() {
            return;
        }

        match self.channel {
            0 => {
                // Control packet
                fields.insert("log_type".into(), "control".into());
                if !self.payload.is_empty() {
                    let cmd = self.payload[0];
                    fields.insert("command".into(), logging_command_label(cmd).to_string());
                }
            }
            1 => {
                // Data packet
                fields.insert("log_type".into(), "data".into());
                if self.payload.len() >= 2 {
                    let log_channel = self.payload[0];
                    let log_id = self.payload[1];
                    fields.insert("log_channel".into(), format!("0x{:02X}", log_channel));
                    fields.insert("log_id".into(), log_id.to_string());
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }

    /// Extract semantic fields from High-Level Commander port packets.
    /// Channel 0: Trajectory commands
    fn extract_high_level_commander_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.is_empty() {
            return;
        }

        match self.channel {
            0 => {
                // Trajectory command
                fields.insert("command_type".into(), "trajectory".into());
                if self.payload.len() >= 1 {
                    let cmd = self.payload[0];
                    fields.insert("command".into(), high_level_command_label(cmd).to_string());
                }
                if self.payload.len() >= 13 {
                    // Typical trajectory: x(4), y(4), z(4), yaw(4)
                    let x = f32::from_le_bytes([
                        self.payload[1],
                        self.payload[2],
                        self.payload[3],
                        self.payload[4],
                    ]);
                    let y = f32::from_le_bytes([
                        self.payload[5],
                        self.payload[6],
                        self.payload[7],
                        self.payload[8],
                    ]);
                    let z = f32::from_le_bytes([
                        self.payload[9],
                        self.payload[10],
                        self.payload[11],
                        self.payload[12],
                    ]);
                    fields.insert("x".into(), format!("{:.2} m", x));
                    fields.insert("y".into(), format!("{:.2} m", y));
                    fields.insert("z".into(), format!("{:.2} m", z));
                }
                if self.payload.len() >= 17 {
                    // Optional yaw field
                    let yaw = f32::from_le_bytes([
                        self.payload[13],
                        self.payload[14],
                        self.payload[15],
                        self.payload[16],
                    ]);
                    fields.insert(
                        "yaw".into(),
                        format!(
                            "{:.2} rad ({:.1}°)",
                            yaw,
                            yaw * 180.0 / std::f32::consts::PI
                        ),
                    );
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }

    /// Extract semantic fields from Setting port packets.
    fn extract_setting_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.is_empty() {
            return;
        }

        let first_byte = self.payload[0];
        match self.channel {
            0 => {
                fields.insert("operation".into(), "get".into());
                fields.insert("setting_id".into(), format!("0x{:02X}", first_byte));
            }
            1 => {
                fields.insert("operation".into(), "set".into());
                fields.insert("setting_id".into(), format!("0x{:02X}", first_byte));
                if self.payload.len() > 1 {
                    let value = self.payload[1];
                    fields.insert("value".into(), value.to_string());
                }
            }
            _ => {
                fields.insert("channel".into(), self.channel.to_string());
            }
        }
    }
}

// Commander command labels
fn commander_command_label(cmd: u8) -> &'static str {
    match cmd {
        0 => "stop",
        1 => "start",
        2 => ".land",
        3 => "takeoff",
        4 => "emergency_stop",
        _ => "unknown",
    }
}

// Memory command labels
fn memory_command_label(cmd: u8) -> &'static str {
    match cmd {
        0 => "read_info",
        1 => "read",
        2 => "write",
        3 => "get_info",
        _ => "unknown",
    }
}

// Memory status labels
fn memory_status_label(status: u8) -> &'static str {
    match status {
        0 => "ok",
        1 => "error",
        2 => "not_found",
        _ => "unknown",
    }
}

// Logging command labels
fn logging_command_label(cmd: u8) -> &'static str {
    match cmd {
        0 => "stop",
        1 => "start",
        _ => "unknown",
    }
}

// High-level commander command labels
fn high_level_command_label(cmd: u8) -> &'static str {
    match cmd {
        0 => "stop",
        1 => "go_to",
        2 => "trajectory",
        3 => "takeoff",
        4 => "land",
        5 => "emergency_stop",
        6 => "hover",
        7 => "zeroer",
        _ => "unknown",
    }
}

/// Streaming CRTP-over-serial decoder with CRC validation.
pub struct CrtpDecoder {
    buffer: Vec<u8>,
    #[allow(dead_code)]
    max_buffer_bytes: usize,
}

impl CrtpDecoder {
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<CrtpPacket> {
        self.buffer.extend_from_slice(chunk);
        let mut packets = Vec::new();

        loop {
            if self.buffer.len() < CRTP_MIN_FRAME_LEN {
                break;
            }

            match try_decode_crtp(&self.buffer) {
                DecodeResult::Packet(packet, frame_len) => {
                    self.buffer.drain(..frame_len);
                    packets.push(packet);
                }
                DecodeResult::NeedMore => break,
                DecodeResult::NoMatch => {
                    // Skip one byte and try again
                    self.buffer.drain(..1);
                    if self.buffer.len() > self.max_buffer_bytes {
                        let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes);
                        self.buffer.drain(..drain.max(1));
                        break;
                    }
                }
            }
        }

        packets
    }
}

enum DecodeResult {
    Packet(CrtpPacket, usize),
    NeedMore,
    NoMatch,
}

fn try_decode_crtp(buffer: &[u8]) -> DecodeResult {
    if buffer.len() < CRTP_MIN_FRAME_LEN {
        return DecodeResult::NeedMore;
    }

    let header = buffer[0];
    let length = buffer[1] as usize;

    if length > 63 {
        return DecodeResult::NoMatch;
    }

    let frame_len = CRTP_HEADER_LEN + CRTP_LENGTH_LEN + length + CRTP_CRC_LEN;
    if buffer.len() < frame_len {
        return DecodeResult::NeedMore;
    }

    let crc_data = &buffer[..frame_len - CRTP_CRC_LEN];
    let received_crc = buffer[frame_len - 1];
    let computed_crc = crc8_update(0, crc_data);

    if computed_crc != received_crc {
        return DecodeResult::NoMatch;
    }

    let port = CrtpPort::from_header(header);
    let channel = header & 0x03;
    let payload = buffer[CRTP_HEADER_LEN + CRTP_LENGTH_LEN..frame_len - CRTP_CRC_LEN].to_vec();

    let packet = CrtpPacket {
        port,
        channel,
        payload,
    };

    DecodeResult::Packet(packet, frame_len)
}

/// CRC-8/SAE-J1850 used by CRTP.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_crtp_frame(port: u8, channel: u8, payload: &[u8]) -> Vec<u8> {
        let header = ((port & 0x07) << 5) | (channel & 0x03);
        let mut frame = vec![header, payload.len() as u8];
        frame.extend_from_slice(payload);
        let crc = crc8_update(0, &frame);
        frame.push(crc);
        frame
    }

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
    fn parameter_read_semantic_fields() {
        // Parameter read request: id(2)
        let packet = CrtpPacket {
            port: CrtpPort::Parameter,
            channel: 0,
            payload: vec![0x01, 0x02], // param_id: 0x0201
        };

        let fields = packet.fields();
        assert_eq!(fields.get("operation").unwrap(), "read");
        assert_eq!(fields.get("param_id").unwrap(), "513"); // 0x0201 = 513
    }

    #[test]
    fn parameter_write_semantic_fields() {
        // Parameter write request: id(2), value(4 as int32)
        let packet = CrtpPacket {
            port: CrtpPort::Parameter,
            channel: 1,
            payload: vec![0x01, 0x02, 0x0A, 0x00, 0x00, 0x00], // param_id: 0x0201, value: 10
        };

        let fields = packet.fields();
        assert_eq!(fields.get("operation").unwrap(), "write");
        assert_eq!(fields.get("param_id").unwrap(), "513");
        assert!(fields.get("param_value").unwrap().contains("10"));
    }

    #[test]
    fn commander_rpyt_semantic_fields() {
        // Commander RPYT: roll(4), pitch(4), yaw(4), thrust(4) = 16 bytes
        let mut payload = vec![0u8; 16];
        // roll: 0.1
        let roll_bytes = 0.1f32.to_le_bytes();
        payload[0..4].copy_from_slice(&roll_bytes);
        // pitch: -0.2
        let pitch_bytes = (-0.2f32).to_le_bytes();
        payload[4..8].copy_from_slice(&pitch_bytes);
        // yaw: 0.0
        let yaw_bytes = 0.0f32.to_le_bytes();
        payload[8..12].copy_from_slice(&yaw_bytes);
        // thrust: 0.5
        let thrust_bytes = 0.5f32.to_le_bytes();
        payload[12..16].copy_from_slice(&thrust_bytes);

        let packet = CrtpPacket {
            port: CrtpPort::Commander,
            channel: 0,
            payload,
        };

        let fields = packet.fields();
        assert_eq!(fields.get("control_mode").unwrap(), "rpyt");
        assert!(fields.get("roll").unwrap().contains("0.100"));
        assert!(fields.get("pitch").unwrap().contains("-0.200"));
        assert!(fields.get("yaw").unwrap().contains("0.000"));
        assert!(fields.get("thrust").unwrap().contains("0.500"));
    }

    #[test]
    fn high_level_commander_trajectory_semantic_fields() {
        // High-level commander trajectory: command(1), x(4), y(4), z(4) = 13 bytes
        let mut payload = vec![0u8; 13];
        payload[0] = 1; // command: go_to
        // x: 1.5
        let x_bytes = 1.5f32.to_le_bytes();
        payload[1..5].copy_from_slice(&x_bytes);
        // y: 2.0
        let y_bytes = 2.0f32.to_le_bytes();
        payload[5..9].copy_from_slice(&y_bytes);
        // z: 0.8
        let z_bytes = 0.8f32.to_le_bytes();
        payload[9..13].copy_from_slice(&z_bytes);

        let packet = CrtpPacket {
            port: CrtpPort::HighLevelCommander,
            channel: 0,
            payload,
        };

        let fields = packet.fields();
        assert_eq!(fields.get("command_type").unwrap(), "trajectory");
        assert_eq!(fields.get("command").unwrap(), "go_to");
        assert!(fields.get("x").unwrap().contains("1.50"));
        assert!(fields.get("y").unwrap().contains("2.00"));
        assert!(fields.get("z").unwrap().contains("0.80"));
    }

    #[test]
    fn logging_data_semantic_fields() {
        // Logging data: log_channel(1), log_id(1)
        let packet = CrtpPacket {
            port: CrtpPort::Logging,
            channel: 1,
            payload: vec![0x01, 0x02], // log_channel: 0x01, log_id: 2
        };

        let fields = packet.fields();
        assert_eq!(fields.get("log_type").unwrap(), "data");
        assert_eq!(fields.get("log_channel").unwrap(), "0x01");
        assert_eq!(fields.get("log_id").unwrap(), "2");
    }
}
