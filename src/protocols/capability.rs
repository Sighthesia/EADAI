//! Unified capability events across protocols.
//!
//! The capability layer maps protocol-specific semantic fields into
//! cross-protocol business capabilities. This allows the UI to consume
//! standardized events (e.g., `Attitude`, `BatteryStatus`, `GpsPosition`)
//! regardless of whether the source is MAVLink or CRTP.

use crate::protocols::crtp::{CrtpPacket, CrtpPort};
use crate::protocols::mavlink::MavlinkPacket;
use serde::Serialize;
use std::collections::BTreeMap;

/// A cross-protocol capability event.
///
/// Each variant represents a domain concept that may be produced by
/// multiple protocols (e.g., attitude data from MAVLink ATTITUDE or
/// CRTP sensor packets).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "capability", rename_all = "snake_case")]
pub enum CapabilityEvent {
    /// Aircraft attitude (roll, pitch, yaw).
    Attitude(AttitudeData),
    /// Battery status (voltage, current, remaining).
    BatteryStatus(BatteryData),
    /// GPS position fix.
    GpsPosition(GpsData),
    /// IMU sensor data (accelerometer, gyroscope, magnetometer).
    ImuData(ImuData),
    /// Position/velocity in local frame.
    LocalPosition(LocalPositionData),
    /// System status heartbeat / health.
    SystemStatus(SystemStatusData),
    /// Raw protocol packet for debug/protocol-specific display.
    RawPacket(RawPacketData),
}

/// Attitude data (roll, pitch, yaw in radians).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AttitudeData {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub rollspeed: Option<f32>,
    pub pitchspeed: Option<f32>,
    pub yawspeed: Option<f32>,
    pub source_protocol: &'static str,
}

/// Battery status.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BatteryData {
    pub voltage_mv: Option<u32>,
    pub current_ma: Option<i32>,
    pub remaining_percent: Option<i8>,
    pub source_protocol: &'static str,
}

/// GPS position.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GpsData {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
    pub satellites: Option<u8>,
    pub fix_type: Option<String>,
    pub source_protocol: &'static str,
}

/// IMU sensor data.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ImuData {
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,
    pub mag_x: Option<f32>,
    pub mag_y: Option<f32>,
    pub mag_z: Option<f32>,
    pub source_protocol: &'static str,
}

/// Local position/velocity.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalPositionData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: Option<f32>,
    pub vy: Option<f32>,
    pub vz: Option<f32>,
    pub source_protocol: &'static str,
}

/// System status.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SystemStatusData {
    pub system_id: u8,
    pub component_id: u8,
    pub status: String,
    pub custom_mode: Option<u32>,
    pub source_protocol: &'static str,
}

/// Raw protocol packet for debug display.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RawPacketData {
    pub protocol: &'static str,
    pub fields: BTreeMap<String, String>,
}

/// Extracts capability events from a MAVLink packet.
///
/// Returns zero or more capability events. A single MAVLink packet
/// may produce multiple capabilities (e.g., SYS_STATUS produces both
/// battery and system status).
pub fn mavlink_to_capabilities(packet: &MavlinkPacket) -> Vec<CapabilityEvent> {
    let mut events = Vec::new();

    match packet.message_id {
        // HEARTBEAT -> SystemStatus
        0x0000 => {
            if let Some(fields) = try_extract_fields(packet) {
                events.push(CapabilityEvent::SystemStatus(SystemStatusData {
                    system_id: packet.system_id,
                    component_id: packet.component_id,
                    status: fields.get("system_status").cloned().unwrap_or_default(),
                    custom_mode: fields.get("custom_mode").and_then(|v| v.parse().ok()),
                    source_protocol: "mavlink",
                }));
            }
        }
        // SYS_STATUS -> BatteryStatus + SystemStatus
        0x0001 => {
            if let Some(fields) = try_extract_fields(packet) {
                events.push(CapabilityEvent::BatteryStatus(BatteryData {
                    voltage_mv: fields
                        .get("voltage_battery")
                        .and_then(|v| v.strip_suffix(" mV").and_then(|s| s.parse().ok())),
                    current_ma: fields
                        .get("current_battery")
                        .and_then(|v| v.strip_suffix(" mA").and_then(|s| s.parse().ok())),
                    remaining_percent: fields
                        .get("battery_remaining")
                        .and_then(|v| v.strip_suffix('%').and_then(|s| s.parse().ok())),
                    source_protocol: "mavlink",
                }));
                events.push(CapabilityEvent::SystemStatus(SystemStatusData {
                    system_id: packet.system_id,
                    component_id: packet.component_id,
                    status: "active".into(),
                    custom_mode: None,
                    source_protocol: "mavlink",
                }));
            }
        }
        // ATTITUDE -> Attitude
        0x00CA => {
            if let Some(fields) = try_extract_fields(packet) {
                if let (Some(roll), Some(pitch), Some(yaw)) = (
                    parse_rad_field(fields.get("roll")),
                    parse_rad_field(fields.get("pitch")),
                    parse_rad_field(fields.get("yaw")),
                ) {
                    events.push(CapabilityEvent::Attitude(AttitudeData {
                        roll,
                        pitch,
                        yaw,
                        rollspeed: fields
                            .get("rollspeed")
                            .and_then(|v| v.strip_suffix(" rad/s").and_then(|s| s.parse().ok())),
                        pitchspeed: fields
                            .get("pitchspeed")
                            .and_then(|v| v.strip_suffix(" rad/s").and_then(|s| s.parse().ok())),
                        yawspeed: fields
                            .get("yawspeed")
                            .and_then(|v| v.strip_suffix(" rad/s").and_then(|s| s.parse().ok())),
                        source_protocol: "mavlink",
                    }));
                }
            }
        }
        // HIGHRES_IMU -> ImuData
        0x00BE => {
            if let Some(fields) = try_extract_fields(packet) {
                let ax = parse_ms2_field(fields.get("xacc"));
                let ay = parse_ms2_field(fields.get("yacc"));
                let az = parse_ms2_field(fields.get("zacc"));
                let gx = parse_rads_field(fields.get("xgyro"));
                let gy = parse_rads_field(fields.get("ygyro"));
                let gz = parse_rads_field(fields.get("zgyro"));
                events.push(CapabilityEvent::ImuData(ImuData {
                    accel_x: ax,
                    accel_y: ay,
                    accel_z: az,
                    gyro_x: gx,
                    gyro_y: gy,
                    gyro_z: gz,
                    mag_x: fields
                        .get("xmag")
                        .and_then(|v| v.strip_suffix(" Gauss").and_then(|s| s.parse().ok())),
                    mag_y: fields
                        .get("ymag")
                        .and_then(|v| v.strip_suffix(" Gauss").and_then(|s| s.parse().ok())),
                    mag_z: fields
                        .get("zmag")
                        .and_then(|v| v.strip_suffix(" Gauss").and_then(|s| s.parse().ok())),
                    source_protocol: "mavlink",
                }));
            }
        }
        // GLOBAL_POSITION_INT -> GpsPosition + LocalPosition
        0x0039 => {
            if let Some(fields) = try_extract_fields(packet) {
                if let (Some(lat), Some(lon), Some(alt)) = (
                    parse_deg_field(fields.get("lat")),
                    parse_deg_field(fields.get("lon")),
                    parse_m_field(fields.get("alt")),
                ) {
                    events.push(CapabilityEvent::GpsPosition(GpsData {
                        latitude_deg: lat,
                        longitude_deg: lon,
                        altitude_m: alt,
                        satellites: fields
                            .get("satellites_visible")
                            .and_then(|v| v.parse().ok()),
                        fix_type: fields.get("fix_type").cloned(),
                        source_protocol: "mavlink",
                    }));
                }
                if let Some(rel_alt) = fields
                    .get("relative_alt")
                    .and_then(|v| v.strip_suffix(" m").and_then(|s| s.parse::<f64>().ok()))
                {
                    events.push(CapabilityEvent::LocalPosition(LocalPositionData {
                        x: 0.0,
                        y: 0.0,
                        z: rel_alt as f32, // relative_alt is the z component in NED
                        vx: None,
                        vy: None,
                        vz: None,
                        source_protocol: "mavlink",
                    }));
                }
            }
        }
        // LOCAL_POSITION_NED -> LocalPosition
        0x0035 => {
            if let Some(fields) = try_extract_fields(packet) {
                events.push(CapabilityEvent::LocalPosition(LocalPositionData {
                    x: fields
                        .get("x")
                        .and_then(|v| v.strip_suffix(" m").and_then(|s| s.parse().ok()))
                        .unwrap_or(0.0),
                    y: fields
                        .get("y")
                        .and_then(|v| v.strip_suffix(" m").and_then(|s| s.parse().ok()))
                        .unwrap_or(0.0),
                    z: fields
                        .get("z")
                        .and_then(|v| v.strip_suffix(" m").and_then(|s| s.parse().ok()))
                        .unwrap_or(0.0),
                    vx: fields
                        .get("vx")
                        .and_then(|v| v.strip_suffix(" m/s").and_then(|s| s.parse().ok())),
                    vy: fields
                        .get("vy")
                        .and_then(|v| v.strip_suffix(" m/s").and_then(|s| s.parse().ok())),
                    vz: fields
                        .get("vz")
                        .and_then(|v| v.strip_suffix(" m/s").and_then(|s| s.parse().ok())),
                    source_protocol: "mavlink",
                }));
            }
        }
        // GPS_RAW_INT -> GpsPosition
        0x0021 => {
            if let Some(fields) = try_extract_fields(packet) {
                if let (Some(lat), Some(lon), Some(alt)) = (
                    parse_deg_field(fields.get("lat")),
                    parse_deg_field(fields.get("lon")),
                    parse_m_field(fields.get("alt")),
                ) {
                    events.push(CapabilityEvent::GpsPosition(GpsData {
                        latitude_deg: lat,
                        longitude_deg: lon,
                        altitude_m: alt,
                        satellites: fields
                            .get("satellites_visible")
                            .and_then(|v| v.parse().ok()),
                        fix_type: fields.get("fix_type").cloned(),
                        source_protocol: "mavlink",
                    }));
                }
            }
        }
        _ => {}
    }

    // Always emit a raw packet event for debug display
    events.push(CapabilityEvent::RawPacket(RawPacketData {
        protocol: "mavlink",
        fields: packet.fields(),
    }));

    events
}

/// Extracts capability events from a CRTP packet.
///
/// Maps CRTP port-specific semantic fields to cross-protocol capability events:
/// - Commander port (0x2) channel 0: RPYT control → AttitudeData
/// - Logging port (0x4) channel 1: log data → RawPacket (requires device-specific log config for ImuData)
/// - All other ports: RawPacket for debug display.
pub fn crtp_to_capabilities(packet: &CrtpPacket) -> Vec<CapabilityEvent> {
    let mut events = Vec::new();

    match &packet.port {
        CrtpPort::Commander if packet.channel == 0 && packet.payload.len() >= 12 => {
            // Commander RPYT: roll(4), pitch(4), yaw(4) as f32 LE
            let roll = f32::from_le_bytes([
                packet.payload[0],
                packet.payload[1],
                packet.payload[2],
                packet.payload[3],
            ]);
            let pitch = f32::from_le_bytes([
                packet.payload[4],
                packet.payload[5],
                packet.payload[6],
                packet.payload[7],
            ]);
            let yaw = f32::from_le_bytes([
                packet.payload[8],
                packet.payload[9],
                packet.payload[10],
                packet.payload[11],
            ]);
            events.push(CapabilityEvent::Attitude(AttitudeData {
                roll,
                pitch,
                yaw,
                rollspeed: None,
                pitchspeed: None,
                yawspeed: None,
                source_protocol: "crtp",
            }));
        }
        _ => {}
    }

    // Always emit a raw packet event for debug display
    events.push(CapabilityEvent::RawPacket(RawPacketData {
        protocol: "crtp",
        fields: packet.fields(),
    }));

    events
}

// --- Helper functions ---

fn try_extract_fields(packet: &MavlinkPacket) -> Option<BTreeMap<String, String>> {
    let fields = packet.fields();
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Parse a field value like "0.50 rad (28.6°)" -> 0.50
fn parse_rad_field(value: Option<&String>) -> Option<f32> {
    let s = value?;
    let num_str = s.split_whitespace().next()?;
    num_str.parse().ok()
}

/// Parse a field value like "9.81 m/s²" -> 9.81
fn parse_ms2_field(value: Option<&String>) -> f32 {
    value
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

/// Parse a field value like "1.23 rad/s" -> 1.23
fn parse_rads_field(value: Option<&String>) -> f32 {
    value
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

/// Parse a field value like "47.5 deg" -> 47.5
fn parse_deg_field(value: Option<&String>) -> Option<f64> {
    let s = value?;
    let num_str = s.split_whitespace().next()?;
    num_str.parse().ok()
}

/// Parse a field value like "12.3 m" -> 12.3
fn parse_m_field(value: Option<&String>) -> Option<f64> {
    let s = value?;
    let num_str = s.split_whitespace().next()?;
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mavlink_attitude_produces_capability() {
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x00CA,
            payload: {
                let mut p = vec![0u8; 32];
                // roll = 0.5 rad
                p[8..12].copy_from_slice(&0.5f32.to_le_bytes());
                // pitch = -0.3 rad
                p[12..16].copy_from_slice(&(-0.3f32).to_le_bytes());
                // yaw = 1.2 rad
                p[16..20].copy_from_slice(&1.2f32.to_le_bytes());
                p
            },
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let events = mavlink_to_capabilities(&packet);
        let attitude = events.iter().find_map(|e| match e {
            CapabilityEvent::Attitude(a) => Some(a),
            _ => None,
        });
        assert!(attitude.is_some());
        let a = attitude.unwrap();
        assert!((a.roll - 0.5).abs() < 0.01);
        assert!((a.pitch - (-0.3)).abs() < 0.01);
        assert!((a.yaw - 1.2).abs() < 0.01);
        assert_eq!(a.source_protocol, "mavlink");
    }

    #[test]
    fn mavlink_heartbeat_produces_system_status() {
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x0000,
            payload: vec![0x02, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x04, 0x03],
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let events = mavlink_to_capabilities(&packet);
        let sys_status = events.iter().find_map(|e| match e {
            CapabilityEvent::SystemStatus(s) => Some(s),
            _ => None,
        });
        assert!(sys_status.is_some());
        assert_eq!(sys_status.unwrap().source_protocol, "mavlink");
    }

    #[test]
    fn raw_packet_always_present() {
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x9999, // unknown message
            payload: vec![],
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let events = mavlink_to_capabilities(&packet);
        let raw = events
            .iter()
            .find(|e| matches!(e, CapabilityEvent::RawPacket(_)));
        assert!(raw.is_some());
    }
}
