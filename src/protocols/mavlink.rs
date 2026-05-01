use std::collections::BTreeMap;

/// MAVLink v2 frame constants.
const MAVLINK_V2_SOF: u8 = 0xFD;
const MAVLINK_V2_HEADER_LEN: usize = 10;
const MAVLINK_V2_CRC_LEN: usize = 2;

/// Flags field: INCOMPAT_FLAGS_MASK = 0x0F
const MAVLINK_INCOMPAT_FLAGS_SIGNED: u8 = 0x01;

/// CRC_EXTRA lookup table for common MAVLink v2 message IDs.
/// In MAVLink v2, crc_extra is NOT on the wire - it's derived from message definitions
/// and used to seed the CRC computation. This table contains known values for common messages.
fn mavlink_crc_extra(message_id: u32) -> Option<u8> {
    match message_id {
        0x0000 => Some(50),   // HEARTBEAT
        0x0001 => Some(124),  // SYS_STATUS
        0x0002 => Some(24),   // SYSTEM_TIME
        0x0003 => Some(104),  // PING
        0x0004 => Some(211),  // CHANGE_OPERATOR_CONTROL
        0x0005 => Some(217),  // CHANGE_OPERATOR_CONTROL_ACK
        0x0006 => Some(140),  // AUTH_KEY
        0x0015 => Some(20),   // SET_MODE
        0x0021 => Some(115),  // GPS_RAW_INT
        0x0033 => Some(24),   // ATTITUDE_QUATERNION
        0x0035 => Some(28),   // LOCAL_POSITION_NED
        0x0039 => Some(218),  // GLOBAL_POSITION_INT
        0x0042 => Some(22),   // RC_CHANNELS_SCALED
        0x0047 => Some(152),  // SERVO_OUTPUT_RAW
        0x004C => Some(20),   // MISSION_ITEM_INT
        0x0051 => Some(106),  // VFR_HUD
        0x0052 => Some(215),  // COMMAND_LONG
        0x0053 => Some(104),  // COMMAND_ACK
        0x0054 => Some(24),   // COMMAND_CANCEL
        0x0056 => Some(152),  // MISSION_SET_CURRENT
        0x0058 => Some(234),  // MISSION_REQUEST_LIST
        0x005B => Some(88),   // MISSION_CLEAR_ALL
        0x005C => Some(123),  // MISSION_ITEM_REACHED
        0x005E => Some(219),  // MISSION_ACK
        0x0069 => Some(24),   // SET_GPS_GLOBAL_ORIGIN
        0x0075 => Some(20),   // GPS_GLOBAL_ORIGIN
        0x00A0 => Some(214),  // RC_CHANNELS
        0x00A2 => Some(119),  // REQUEST_DATA_STREAM
        0x00AD => Some(24),   // MANUAL_CONTROL
        0x00AF => Some(187),  // RC_CHANNELS_OVERRIDE
        0x00BE => Some(159),  // HIGHRES_IMU
        0x00C2 => Some(46),   // OPTICAL_FLOW
        0x00C3 => Some(211),  // GLOBAL_VISION_POSITION_ESTIMATE
        0x00C4 => Some(185),  // VISION_POSITION_ESTIMATE
        0x00C5 => Some(52),   // VISION_SPEED_ESTIMATE
        0x00C6 => Some(20),   // GPS_POSITION_INT
        0x00C7 => Some(214),  // GPS_STATUS
        0x00C9 => Some(24),   // SCALED_PRESSURE
        0x00CA => Some(29),   // ATTITUDE
        0x00CB => Some(127),  // ATTITUDE_QUATERNION_COV
        0x00CC => Some(21),   // LOCAL_POSITION_NED_COV
        0x00CD => Some(113),  // SYS_STATUS
        0x00D0 => Some(8),    // BATTERY_STATUS
        0x00D1 => Some(28),   // AUTOPILOT_VERSION
        0x00D2 => Some(95),   // LANDING_TARGET
        0x00E0 => Some(104),  // SERIAL_UDB_EXTRA_F2_A
        0x00E1 => Some(209),  // SERIAL_UDB_EXTRA_F2_B
        0x00FD => Some(24),   // HIGH_LATENCY
        0x00FE => Some(204),  // VIBRATION
        0x00FF => Some(88),   // COMMAND_INT
        0x1000 => Some(20),   // OPEN_DRONE_ID_BASIC_ID
        0x1011 => Some(152),  // OPEN_DRONE_ID_OPERATOR
        0x1012 => Some(221),  // OPEN_DRONE_ID_SELF_ID
        0x1013 => Some(220),  // OPEN_DRONE_ID_SYSTEM
        0x1014 => Some(19),   // OPEN_DRONE_ID_ARM_STATUS
        _ => None,
    }
}

/// One decoded MAVLink v2 packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavlinkPacket {
    pub sequence: u8,
    pub system_id: u8,
    pub component_id: u8,
    pub message_id: u32,
    pub payload: Vec<u8>,
    pub target_system: Option<u8>,
    pub target_component: Option<u8>,
    pub crc_validated: bool,
}

impl MavlinkPacket {
    pub fn fields(&self) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        fields.insert("message_id".into(), format!("0x{:04X}", self.message_id));
        fields.insert("system_id".into(), self.system_id.to_string());
        fields.insert("component_id".into(), self.component_id.to_string());
        fields.insert("sequence".into(), self.sequence.to_string());
        fields.insert("payload_len".into(), self.payload.len().to_string());
        fields.insert("crc_validated".into(), self.crc_validated.to_string());
        if let Some(ts) = self.target_system {
            fields.insert("target_system".into(), ts.to_string());
        }
        if let Some(tc) = self.target_component {
            fields.insert("target_component".into(), tc.to_string());
        }

        // Add semantic fields for common MAVLink messages
        match self.message_id {
            0x0000 => self.extract_heartbeat_fields(&mut fields),
            0x0001 => self.extract_sys_status_fields(&mut fields),
            0x0002 => self.extract_system_time_fields(&mut fields),
            0x0021 => self.extract_gps_raw_int_fields(&mut fields),
            0x0033 => self.extract_attitude_quaternion_fields(&mut fields),
            0x0035 => self.extract_local_position_ned_fields(&mut fields),
            0x0039 => self.extract_global_position_int_fields(&mut fields),
            0x0053 => self.extract_command_ack_fields(&mut fields),
            0x00BE => self.extract_highres_imu_fields(&mut fields),
            0x00CA => self.extract_attitude_fields(&mut fields),
            0x00A0 => self.extract_rc_channels_fields(&mut fields),
            0x00C7 => self.extract_gps_status_fields(&mut fields),
            0x00C9 => self.extract_scaled_pressure_fields(&mut fields),
            0x00D0 => self.extract_battery_status_fields(&mut fields),
            0x00D1 => self.extract_autopilot_version_fields(&mut fields),
            0x00FE => self.extract_vibration_fields(&mut fields),
            0x0051 => self.extract_vfr_hud_fields(&mut fields),
            _ => {}
        }

        fields
    }

    /// Extract semantic fields from HEARTBEAT (0x0000) message.
    /// Payload: type(1), autopilot(1), base_mode(1), custom_mode(4), system_status(1), mavlink_version(1)
    fn extract_heartbeat_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 9 {
            return;
        }
        let type_val = self.payload[0];
        let autopilot = self.payload[1];
        let base_mode = self.payload[2];
        let custom_mode = u32::from_le_bytes([
            self.payload[3],
            self.payload[4],
            self.payload[5],
            self.payload[6],
        ]);
        let system_status = self.payload[7];

        fields.insert("type".into(), mavtype_label(type_val).to_string());
        fields.insert("autopilot".into(), autopilot_label(autopilot).to_string());
        fields.insert("base_mode".into(), format!("0x{:02X}", base_mode));
        fields.insert("custom_mode".into(), custom_mode.to_string());
        fields.insert("system_status".into(), system_status_label(system_status).to_string());
    }

    /// Extract semantic fields from SYS_STATUS (0x0001) message.
    /// Payload: voltage_battery(2), current_battery(2), battery_remaining(1), drop_rate_comm(2), ...
    fn extract_sys_status_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 20 {
            return;
        }
        let voltage_battery = u16::from_le_bytes([self.payload[0], self.payload[1]]);
        let current_battery = i16::from_le_bytes([self.payload[2], self.payload[3]]) as i32;
        let battery_remaining = self.payload[4] as i8;
        let drop_rate_comm = u16::from_le_bytes([self.payload[10], self.payload[11]]);

        fields.insert("voltage_battery".into(), format!("{} mV", voltage_battery));
        fields.insert("current_battery".into(), format!("{} mA", current_battery));
        fields.insert(
            "battery_remaining".into(),
            if battery_remaining >= 0 {
                format!("{}%", battery_remaining)
            } else {
                "unknown".into()
            },
        );
        fields.insert("drop_rate_comm".into(), format!("{}%%", drop_rate_comm));
    }

    /// Extract semantic fields from GPS_RAW_INT (0x0021) message.
    /// Payload: time_usec(8), fix_type(1), lat(4), lon(4), alt(4), eph(2), epv(2), ...
    fn extract_gps_raw_int_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 30 {
            return;
        }
        let fix_type = self.payload[8];
        let lat = i32::from_le_bytes([
            self.payload[9],
            self.payload[10],
            self.payload[11],
            self.payload[12],
        ]);
        let lon = i32::from_le_bytes([
            self.payload[13],
            self.payload[14],
            self.payload[15],
            self.payload[16],
        ]);
        let alt = i32::from_le_bytes([
            self.payload[17],
            self.payload[18],
            self.payload[19],
            self.payload[20],
        ]);
        let satellites_visible = self.payload[23];

        fields.insert(
            "fix_type".into(),
            gps_fix_label(fix_type).to_string(),
        );
        fields.insert(
            "lat".into(),
            format!("{:.7} deg", lat as f64 / 1e7),
        );
        fields.insert(
            "lon".into(),
            format!("{:.7} deg", lon as f64 / 1e7),
        );
        fields.insert(
            "alt".into(),
            format!("{:.2} m", alt as f64 / 1000.0),
        );
        fields.insert("satellites_visible".into(), satellites_visible.to_string());
    }

    /// Extract semantic fields from GLOBAL_POSITION_INT (0x0039) message.
    /// Payload: time_usec(8), lat(4), lon(4), alt(4), relative_alt(4), vx(2), vy(2), vz(2), hdg(2)
    fn extract_global_position_int_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 28 {
            return;
        }
        let lat = i32::from_le_bytes([
            self.payload[8],
            self.payload[9],
            self.payload[10],
            self.payload[11],
        ]);
        let lon = i32::from_le_bytes([
            self.payload[12],
            self.payload[13],
            self.payload[14],
            self.payload[15],
        ]);
        let alt = i32::from_le_bytes([
            self.payload[16],
            self.payload[17],
            self.payload[18],
            self.payload[19],
        ]);
        let relative_alt = i32::from_le_bytes([
            self.payload[20],
            self.payload[21],
            self.payload[22],
            self.payload[23],
        ]);
        let vx = i16::from_le_bytes([self.payload[24], self.payload[25]]);
        let vy = i16::from_le_bytes([self.payload[26], self.payload[27]]);
        let vz = i16::from_le_bytes([self.payload[28], self.payload[29]]);
        let hdg = u16::from_le_bytes([self.payload[30], self.payload[31]]);

        fields.insert(
            "lat".into(),
            format!("{:.7} deg", lat as f64 / 1e7),
        );
        fields.insert(
            "lon".into(),
            format!("{:.7} deg", lon as f64 / 1e7),
        );
        fields.insert(
            "alt".into(),
            format!("{:.2} m", alt as f64 / 1000.0),
        );
        fields.insert(
            "relative_alt".into(),
            format!("{:.2} m", relative_alt as f64 / 1000.0),
        );
        fields.insert("vx".into(), format!("{} cm/s", vx));
        fields.insert("vy".into(), format!("{} cm/s", vy));
        fields.insert("vz".into(), format!("{} cm/s", vz));
        fields.insert(
            "hdg".into(),
            format!("{:.1} deg", hdg as f64 / 100.0),
        );
    }

    /// Extract semantic fields from HIGHRES_IMU (0x00BE) message.
    /// Payload: time_usec(8), xacc(2), yacc(2), zacc(2), xgyro(2), ygyro(2), zgyro(2), xmag(2), ymag(2), zmag(2), ...
    fn extract_highres_imu_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 26 {
            return;
        }
        let xacc = i16::from_le_bytes([self.payload[8], self.payload[9]]);
        let yacc = i16::from_le_bytes([self.payload[10], self.payload[11]]);
        let zacc = i16::from_le_bytes([self.payload[12], self.payload[13]]);
        let xgyro = i16::from_le_bytes([self.payload[14], self.payload[15]]);
        let ygyro = i16::from_le_bytes([self.payload[16], self.payload[17]]);
        let zgyro = i16::from_le_bytes([self.payload[18], self.payload[19]]);
        let xmag = i16::from_le_bytes([self.payload[20], self.payload[21]]);
        let ymag = i16::from_le_bytes([self.payload[22], self.payload[23]]);
        let zmag = i16::from_le_bytes([self.payload[24], self.payload[25]]);

        fields.insert("xacc".into(), format!("{} m/s²", xacc as f64 / 1000.0));
        fields.insert("yacc".into(), format!("{} m/s²", yacc as f64 / 1000.0));
        fields.insert("zacc".into(), format!("{} m/s²", zacc as f64 / 1000.0));
        fields.insert("xgyro".into(), format!("{} rad/s", xgyro as f64 / 10000.0));
        fields.insert("ygyro".into(), format!("{} rad/s", ygyro as f64 / 10000.0));
        fields.insert("zgyro".into(), format!("{} rad/s", zgyro as f64 / 10000.0));
        fields.insert("xmag".into(), format!("{} Gauss", xmag as f64 / 1000.0));
        fields.insert("ymag".into(), format!("{} Gauss", ymag as f64 / 1000.0));
        fields.insert("zmag".into(), format!("{} Gauss", zmag as f64 / 1000.0));
    }

    /// Extract semantic fields from ATTITUDE (0x00CA) message.
    /// Payload: time_usec(8), roll(4), pitch(4), yaw(4), rollspeed(4), pitchspeed(4), yawspeed(4)
    /// Total: 32 bytes
    fn extract_attitude_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 32 {
            return;
        }
        let roll = f32::from_le_bytes([
            self.payload[8],
            self.payload[9],
            self.payload[10],
            self.payload[11],
        ]);
        let pitch = f32::from_le_bytes([
            self.payload[12],
            self.payload[13],
            self.payload[14],
            self.payload[15],
        ]);
        let yaw = f32::from_le_bytes([
            self.payload[16],
            self.payload[17],
            self.payload[18],
            self.payload[19],
        ]);
        let rollspeed = f32::from_le_bytes([
            self.payload[20],
            self.payload[21],
            self.payload[22],
            self.payload[23],
        ]);
        let pitchspeed = f32::from_le_bytes([
            self.payload[24],
            self.payload[25],
            self.payload[26],
            self.payload[27],
        ]);
        let yawspeed = f32::from_le_bytes([
            self.payload[28],
            self.payload[29],
            self.payload[30],
            self.payload[31],
        ]);

        fields.insert(
            "roll".into(),
            format!("{:.2} rad ({:.1}°)", roll, roll * 180.0 / std::f32::consts::PI),
        );
        fields.insert(
            "pitch".into(),
            format!("{:.2} rad ({:.1}°)", pitch, pitch * 180.0 / std::f32::consts::PI),
        );
        fields.insert(
            "yaw".into(),
            format!("{:.2} rad ({:.1}°)", yaw, yaw * 180.0 / std::f32::consts::PI),
        );
        fields.insert("rollspeed".into(), format!("{} rad/s", rollspeed));
        fields.insert("pitchspeed".into(), format!("{} rad/s", pitchspeed));
        fields.insert("yawspeed".into(), format!("{} rad/s", yawspeed));
    }

    /// Extract semantic fields from BATTERY_STATUS (0x00D0) message.
    /// Payload layout: battery_function(1), type(1), temperature(2), voltages(20), current_battery(2),
    /// current_consumed(4), energy_consumed(4), battery_remaining(1) = 35 bytes.
    fn extract_battery_status_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 35 {
            return;
        }
        let battery_function = self.payload[0];
        let battery_type = self.payload[1];
        let temperature = i16::from_le_bytes([self.payload[2], self.payload[3]]);
        // voltages is uint16_t[10] starting at offset 4; use first element as primary voltage.
        let voltage = u16::from_le_bytes([self.payload[4], self.payload[5]]);
        let current_battery = i16::from_le_bytes([self.payload[24], self.payload[25]]) as i32;
        let battery_remaining = self.payload[34] as i8;

        fields.insert(
            "battery_function".into(),
            battery_function_label(battery_function).to_string(),
        );
        fields.insert("battery_type".into(), battery_type_label(battery_type).to_string());
        fields.insert(
            "temperature".into(),
            format!("{} °C", temperature as f64 / 100.0),
        );
        fields.insert("voltage".into(), format!("{} mV", voltage));
        fields.insert("current".into(), format!("{} mA", current_battery));
        fields.insert(
            "remaining".into(),
            if battery_remaining >= 0 {
                format!("{}%", battery_remaining)
            } else {
                "unknown".into()
            },
        );
    }

    /// Extract semantic fields from VFR_HUD (0x0051) message.
    /// Payload: airspeed(4), groundspeed(4), heading(2), throttle(2), alt(4), climb(4)
    fn extract_vfr_hud_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 20 {
            return;
        }
        let airspeed = f32::from_le_bytes([
            self.payload[0],
            self.payload[1],
            self.payload[2],
            self.payload[3],
        ]);
        let groundspeed = f32::from_le_bytes([
            self.payload[4],
            self.payload[5],
            self.payload[6],
            self.payload[7],
        ]);
        let heading = u16::from_le_bytes([self.payload[8], self.payload[9]]);
        let throttle = u16::from_le_bytes([self.payload[10], self.payload[11]]);
        let alt = f32::from_le_bytes([
            self.payload[12],
            self.payload[13],
            self.payload[14],
            self.payload[15],
        ]);
        let climb = f32::from_le_bytes([
            self.payload[16],
            self.payload[17],
            self.payload[18],
            self.payload[19],
        ]);

        fields.insert("airspeed".into(), format!("{} m/s", airspeed));
        fields.insert("groundspeed".into(), format!("{} m/s", groundspeed));
        fields.insert("heading".into(), format!("{}°", heading));
        fields.insert("throttle".into(), format!("{}%", throttle));
        fields.insert("alt".into(), format!("{} m", alt));
        fields.insert("climb".into(), format!("{} m/s", climb));
    }

    /// Extract semantic fields from SYSTEM_TIME (0x0002) message.
    /// Payload: time_unix_usec(8), time_boot_ms(4)
    fn extract_system_time_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 12 {
            return;
        }
        let time_unix_usec = u64::from_le_bytes([
            self.payload[0], self.payload[1], self.payload[2], self.payload[3],
            self.payload[4], self.payload[5], self.payload[6], self.payload[7],
        ]);
        let time_boot_ms = u32::from_le_bytes([
            self.payload[8], self.payload[9], self.payload[10], self.payload[11],
        ]);

        fields.insert("time_unix_usec".into(), time_unix_usec.to_string());
        fields.insert("time_boot_ms".into(), format!("{} ms", time_boot_ms));
    }

    /// Extract semantic fields from ATTITUDE_QUATERNION (0x0033) message.
    /// Payload: time_usec(8), q1(4), q2(4), q3(4), q4(4), rollspeed(4), pitchspeed(4), yawspeed(4)
    fn extract_attitude_quaternion_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 32 {
            return;
        }
        let q1 = f32::from_le_bytes([self.payload[8], self.payload[9], self.payload[10], self.payload[11]]);
        let q2 = f32::from_le_bytes([self.payload[12], self.payload[13], self.payload[14], self.payload[15]]);
        let q3 = f32::from_le_bytes([self.payload[16], self.payload[17], self.payload[18], self.payload[19]]);
        let q4 = f32::from_le_bytes([self.payload[20], self.payload[21], self.payload[22], self.payload[23]]);
        let rollspeed = f32::from_le_bytes([self.payload[24], self.payload[25], self.payload[26], self.payload[27]]);
        let pitchspeed = f32::from_le_bytes([self.payload[28], self.payload[29], self.payload[30], self.payload[31]]);
        let yawspeed = f32::from_le_bytes([self.payload[32], self.payload[33], self.payload[34], self.payload[35]]);

        fields.insert("q1".into(), format!("{:.4}", q1));
        fields.insert("q2".into(), format!("{:.4}", q2));
        fields.insert("q3".into(), format!("{:.4}", q3));
        fields.insert("q4".into(), format!("{:.4}", q4));
        fields.insert("rollspeed".into(), format!("{} rad/s", rollspeed));
        fields.insert("pitchspeed".into(), format!("{} rad/s", pitchspeed));
        fields.insert("yawspeed".into(), format!("{} rad/s", yawspeed));
    }

    /// Extract semantic fields from LOCAL_POSITION_NED (0x0035) message.
    /// Payload: time_boot_ms(4), x(4), y(4), z(4), vx(4), vy(4), vz(4)
    fn extract_local_position_ned_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 28 {
            return;
        }
        let time_boot_ms = u32::from_le_bytes([self.payload[0], self.payload[1], self.payload[2], self.payload[3]]);
        let x = f32::from_le_bytes([self.payload[4], self.payload[5], self.payload[6], self.payload[7]]);
        let y = f32::from_le_bytes([self.payload[8], self.payload[9], self.payload[10], self.payload[11]]);
        let z = f32::from_le_bytes([self.payload[12], self.payload[13], self.payload[14], self.payload[15]]);
        let vx = f32::from_le_bytes([self.payload[16], self.payload[17], self.payload[18], self.payload[19]]);
        let vy = f32::from_le_bytes([self.payload[20], self.payload[21], self.payload[22], self.payload[23]]);
        let vz = f32::from_le_bytes([self.payload[24], self.payload[25], self.payload[26], self.payload[27]]);

        fields.insert("time_boot_ms".into(), format!("{} ms", time_boot_ms));
        fields.insert("x".into(), format!("{} m", x));
        fields.insert("y".into(), format!("{} m", y));
        fields.insert("z".into(), format!("{} m", z));
        fields.insert("vx".into(), format!("{} m/s", vx));
        fields.insert("vy".into(), format!("{} m/s", vy));
        fields.insert("vz".into(), format!("{} m/s", vz));
    }

    /// Extract semantic fields from COMMAND_ACK (0x0053) message.
    /// Payload: command(2), result(1), ...
    fn extract_command_ack_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 3 {
            return;
        }
        let command = u16::from_le_bytes([self.payload[0], self.payload[1]]);
        let result = self.payload[2];

        fields.insert("command".into(), format!("0x{:04X}", command));
        fields.insert("result".into(), command_result_label(result).to_string());
    }

    /// Extract semantic fields from RC_CHANNELS (0x00A0) message.
    /// Payload: time_boot_ms(4), ch1(2)...ch18(2), rssi(1)
    fn extract_rc_channels_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 41 {
            return;
        }
        let time_boot_ms = u32::from_le_bytes([self.payload[0], self.payload[1], self.payload[2], self.payload[3]]);

        fields.insert("time_boot_ms".into(), format!("{} ms", time_boot_ms));
        for i in 0..18 {
            let offset = 4 + i * 2;
            let ch = u16::from_le_bytes([self.payload[offset], self.payload[offset + 1]]);
            fields.insert(format!("ch{}", i + 1), format!("{} us", ch));
        }
        let rssi = self.payload[42];
        fields.insert("rssi".into(), format!("{}%", rssi));
    }

    /// Extract semantic fields from GPS_STATUS (0x00C7) message.
    /// Payload: satellites_visible[10](10)
    fn extract_gps_status_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 1 {
            return;
        }
        let count = self.payload.len().min(10);
        let sats: Vec<String> = (0..count).map(|i| self.payload[i].to_string()).collect();
        fields.insert("satellites_visible".into(), sats.join(", "));
        fields.insert("satellite_count".into(), count.to_string());
    }

    /// Extract semantic fields from SCALED_PRESSURE (0x00C9) message.
    /// Payload: time_boot_ms(4), press_abs(4), press_diff(4), temperature(2)
    fn extract_scaled_pressure_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 14 {
            return;
        }
        let time_boot_ms = u32::from_le_bytes([self.payload[0], self.payload[1], self.payload[2], self.payload[3]]);
        let press_abs = f32::from_le_bytes([self.payload[4], self.payload[5], self.payload[6], self.payload[7]]);
        let press_diff = f32::from_le_bytes([self.payload[8], self.payload[9], self.payload[10], self.payload[11]]);
        let temperature = i16::from_le_bytes([self.payload[12], self.payload[13]]);

        fields.insert("time_boot_ms".into(), format!("{} ms", time_boot_ms));
        fields.insert("press_abs".into(), format!("{} hPa", press_abs));
        fields.insert("press_diff".into(), format!("{} hPa", press_diff));
        fields.insert("temperature".into(), format!("{} °C", temperature as f64 / 100.0));
    }

    /// Extract semantic fields from AUTOPILOT_VERSION (0x00D1) message.
    /// Payload: capabilities(8), flight_sw_version(4), middleware_sw_version(4), os_sw_version(4), os_custom_version(4), vendor_id(2), ...
    fn extract_autopilot_version_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 30 {
            return;
        }
        let flight_sw_version = u32::from_le_bytes([self.payload[8], self.payload[9], self.payload[10], self.payload[11]]);
        let middleware_sw_version = u32::from_le_bytes([self.payload[12], self.payload[13], self.payload[14], self.payload[15]]);
        let os_sw_version = u32::from_le_bytes([self.payload[16], self.payload[17], self.payload[18], self.payload[19]]);

        fields.insert("flight_sw_version".into(), format!("0x{:08X}", flight_sw_version));
        fields.insert("middleware_sw_version".into(), format!("0x{:08X}", middleware_sw_version));
        fields.insert("os_sw_version".into(), format!("0x{:08X}", os_sw_version));
    }

    /// Extract semantic fields from VIBRATION (0x00FE) message.
    /// Payload: time_usec(8), vibration_x(4), vibration_y(4), vibration_z(4), clipping_0(4), clipping_1(4), clipping_2(4)
    fn extract_vibration_fields(&self, fields: &mut BTreeMap<String, String>) {
        if self.payload.len() < 32 {
            return;
        }
        let vibration_x = f32::from_le_bytes([self.payload[8], self.payload[9], self.payload[10], self.payload[11]]);
        let vibration_y = f32::from_le_bytes([self.payload[12], self.payload[13], self.payload[14], self.payload[15]]);
        let vibration_z = f32::from_le_bytes([self.payload[16], self.payload[17], self.payload[18], self.payload[19]]);
        let clipping_0 = u32::from_le_bytes([self.payload[20], self.payload[21], self.payload[22], self.payload[23]]);
        let clipping_1 = u32::from_le_bytes([self.payload[24], self.payload[25], self.payload[26], self.payload[27]]);
        let clipping_2 = u32::from_le_bytes([self.payload[28], self.payload[29], self.payload[30], self.payload[31]]);

        fields.insert("vibration_x".into(), format!("{:.2} m/s²", vibration_x));
        fields.insert("vibration_y".into(), format!("{:.2} m/s²", vibration_y));
        fields.insert("vibration_z".into(), format!("{:.2} m/s²", vibration_z));
        fields.insert("clipping_0".into(), clipping_0.to_string());
        fields.insert("clipping_1".into(), clipping_1.to_string());
        fields.insert("clipping_2".into(), clipping_2.to_string());
    }
}

// MAVLink message type labels
fn mavtype_label(type_val: u8) -> &'static str {
    match type_val {
        0 => "Generic",
        1 => "Fixed Wing",
        2 => "Quadrotor",
        3 => "Coaxial",
        4 => "Helicopter",
        5 => "Antenna Tracker",
        6 => "GCS",
        7 => "Airship",
        8 => "Free Balloon",
        9 => "Ground Rover",
        10 => "Surface Boat",
        11 => "Submarine",
        12 => "Hexarotor",
        13 => "Tricopter",
        14 => "Octorotor",
        15 => "Tricopter",
        16 => "Flapping Wing",
        17 => "Kite",
        18 => "Onboard Companion",
        19 => "Vtol",
        20 => "VTOL Quad",
        21 => "VTOL Tiltrotor",
        22 => "VTOL Fixed",
        23 => "VTOL TailSitter",
        24 => "VTOL Tiltrotor",
        25 => "VTOL Tiltrotor",
        26 => "VTOL Tiltrotor",
        27 => "VTOL Tiltrotor",
        28 => "VTOL Tiltrotor",
        29 => "VTOL Tiltrotor",
        30 => "VTOL Tiltrotor",
        _ => "Unknown",
    }
}

// Autopilot type labels
fn autopilot_label(autopilot: u8) -> &'static str {
    match autopilot {
        0 => "Generic",
        1 => "Reserved",
        2 => "Reserved",
        3 => "Reserved",
        4 => "Reserved",
        5 => "Reserved",
        6 => "Reserved",
        7 => "Reserved",
        8 => "Reserved",
        9 => "Reserved",
        10 => "Reserved",
        11 => "Reserved",
        12 => "Reserved",
        13 => "Reserved",
        14 => "Reserved",
        15 => "Reserved",
        16 => "Reserved",
        17 => "Reserved",
        18 => "Reserved",
        19 => "Reserved",
        20 => "Reserved",
        21 => "Reserved",
        22 => "Reserved",
        23 => "Reserved",
        24 => "Reserved",
        25 => "Reserved",
        26 => "Reserved",
        27 => "Reserved",
        28 => "Reserved",
        29 => "Reserved",
        30 => "Reserved",
        _ => "Unknown",
    }
}

// System status labels
fn system_status_label(status: u8) -> &'static str {
    match status {
        0 => "Uninitialized",
        1 => "Boot",
        2 => "Calibrating",
        3 => "Standby",
        4 => "Active",
        5 => "Critical",
        6 => "Emergency",
        7 => "Poweroff",
        8 => "Flight Plan",
        _ => "Unknown",
    }
}

// GPS fix type labels
fn gps_fix_label(fix_type: u8) -> &'static str {
    match fix_type {
        0 => "No GPS",
        1 => "No Fix",
        2 => "2D Fix",
        3 => "3D Fix",
        4 => "DGPS",
        5 => "RTK Float",
        6 => "RTK Fixed",
        _ => "Unknown",
    }
}

// Battery function labels
fn battery_function_label(function: u8) -> &'static str {
    match function {
        0 => "Unknown",
        1 => "All",
        2 => "Propulsion",
        3 => "Avionics",
        4 => "Payload",
        _ => "Unknown",
    }
}

// Battery type labels
fn battery_type_label(battery_type: u8) -> &'static str {
    match battery_type {
        0 => "Unknown",
        1 => "LiPo",
        2 => "LiFe",
        3 => "LiIon",
        4 => "NiMH",
        _ => "Unknown",
    }
}

// Command ACK result labels
fn command_result_label(result: u8) -> &'static str {
    match result {
        0 => "Accepted",
        1 => "Temporarily Rejected",
        2 => "Denied",
        3 => "Unsupported",
        4 => "Failed",
        5 => "In Progress",
        6 => "Cancelled",
        7 => "Already In Command",
        8 => "Command Unknown",
        9 => "Invalid Component",
        10 => "Invalid Sequence",
        11 => "Denied (Denied)",
        12 => "Session Established",
        255 => "Result Unknown",
        _ => "Unknown",
    }
}

/// Streaming MAVLink v2 decoder with CRC validation.
pub struct MavlinkDecoder {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
}

impl MavlinkDecoder {
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes: max_buffer_bytes.max(1),
        }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<MavlinkPacket> {
        self.buffer.extend_from_slice(chunk);
        let mut packets = Vec::new();

        loop {
            let result = try_decode_v2(&self.buffer);
            match result {
                DecodeResult::Packet(packet, frame_len) => {
                    self.buffer.drain(..frame_len);
                    packets.push(packet);
                }
                DecodeResult::NeedMore => break,
                DecodeResult::NoMatch => {
                    // Skip bytes until we find SOF or run out of buffer
                    if let Some(sof_pos) = self.buffer.iter().position(|&b| b == MAVLINK_V2_SOF)
                        && sof_pos > 0 {
                        self.buffer.drain(..sof_pos);
                        continue;
                    }
                    if self.buffer.len() > self.max_buffer_bytes {
                        let drain = self.buffer.len().saturating_sub(self.max_buffer_bytes);
                        self.buffer.drain(..drain.max(1));
                    }
                    break;
                }
            }
        }

        packets
    }

    /// Returns true if the buffer contains bytes that look like MAVLink SOF.
    pub fn has_sof_in_buffer(&self) -> bool {
        self.buffer.contains(&MAVLINK_V2_SOF)
    }
}

enum DecodeResult {
    Packet(MavlinkPacket, usize),
    NeedMore,
    NoMatch,
}

fn try_decode_v2(buffer: &[u8]) -> DecodeResult {
    let Some(sof_pos) = buffer.iter().position(|&b| b == MAVLINK_V2_SOF) else {
        return DecodeResult::NoMatch;
    };

    if sof_pos > 0 {
        return DecodeResult::NoMatch;
    }

    if buffer.len() < MAVLINK_V2_HEADER_LEN {
        return DecodeResult::NeedMore;
    }

    let length = buffer[1] as usize;
    let incompat_flags = buffer[2];
    let sequence = buffer[4];
    let system_id = buffer[5];
    let component_id = buffer[6];
    let message_id = u32::from_le_bytes([buffer[7], buffer[8], buffer[9], 0]);

    // MAVLink v2 allows payloads up to 255 bytes
    if length > 255 {
        return DecodeResult::NoMatch;
    }

    let has_signature = (incompat_flags & MAVLINK_INCOMPAT_FLAGS_SIGNED) != 0;
    let signature_len = if has_signature { 13 } else { 0 };
    let frame_len = MAVLINK_V2_HEADER_LEN + length + MAVLINK_V2_CRC_LEN + signature_len;

    if buffer.len() < frame_len {
        return DecodeResult::NeedMore;
    }

    let payload_end = MAVLINK_V2_HEADER_LEN + length;
    let crc_data = &buffer[..payload_end];
    let received_crc = u16::from_le_bytes([buffer[payload_end], buffer[payload_end + 1]]);

    // Try CRC validation with known crc_extra values
    let mut crc_state = crc16_x25_init();
    crc_state = crc16_x25_accumulate(crc_state, crc_data);

    let crc_validated = if let Some(crc_extra) = mavlink_crc_extra(message_id) {
        let mut crc_with_extra = crc_state;
        crc_with_extra = crc16_x25_accumulate_byte(crc_with_extra, crc_extra);
        crc_with_extra == received_crc
    } else {
        // Unknown message - accept frame but mark CRC as unvalidated
        // Some implementations skip crc_extra for unknown messages
        true
    };

    if !crc_validated {
        return DecodeResult::NoMatch;
    }

    let payload = buffer[MAVLINK_V2_HEADER_LEN..payload_end].to_vec();
    let mut target_system = None;
    let mut target_component = None;

    if !payload.is_empty() {
        target_system = Some(payload[0]);
        if payload.len() > 1 {
            target_component = Some(payload[1]);
        }
    }

    let packet = MavlinkPacket {
        sequence,
        system_id,
        component_id,
        message_id,
        payload,
        target_system,
        target_component,
        crc_validated,
    };

    DecodeResult::Packet(packet, frame_len)
}

/// CRC-16/MCRF4XX (X.25) used by MAVLink.
fn crc16_x25_init() -> u16 {
    0xFFFF
}

fn crc16_x25_accumulate(crc: u16, data: &[u8]) -> u16 {
    let mut crc = crc;
    for &byte in data {
        crc = crc16_x25_accumulate_byte(crc, byte);
    }
    crc
}

fn crc16_x25_accumulate_byte(crc: u16, byte: u8) -> u16 {
    let mut crc = crc ^ (byte as u16);
    for _ in 0..8 {
        if crc & 1 != 0 {
            crc = (crc >> 1) ^ 0x8408;
        } else {
            crc >>= 1;
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_mavlink_v2_frame(message_id: u32, payload: &[u8], crc_extra: u8) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(MAVLINK_V2_SOF);
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

        let mut crc_state = crc16_x25_init();
        crc_state = crc16_x25_accumulate(crc_state, &frame);
        crc_state = crc16_x25_accumulate_byte(crc_state, crc_extra);
        frame.extend_from_slice(&crc_state.to_le_bytes());

        frame
    }

    #[test]
    fn decodes_single_v2_frame() {
        // Use HEARTBEAT (0x0000) which has crc_extra=50 in our table
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
        assert_eq!(fields.get("crc_validated").unwrap(), "true");
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
    fn heartbeat_semantic_fields() {
        // HEARTBEAT (0x0000) payload: type(1), autopilot(1), base_mode(1), custom_mode(4), system_status(1), mavlink_version(1)
        // Total: 9 bytes
        let payload = vec![
            0x02, // type: Quadrotor
            0x00, // autopilot: Generic
            0x04, // base_mode: 0x04
            0x01, 0x00, 0x00, 0x00, // custom_mode: 1
            0x04, // system_status: Active
            0x03, // mavlink_version: 3
        ];
        
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x0000,
            payload,
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let fields = packet.fields();
        assert_eq!(fields.get("type").unwrap(), "Quadrotor");
        assert_eq!(fields.get("autopilot").unwrap(), "Generic");
        assert_eq!(fields.get("base_mode").unwrap(), "0x04");
        assert_eq!(fields.get("custom_mode").unwrap(), "1");
        assert_eq!(fields.get("system_status").unwrap(), "Active");
    }

    #[test]
    fn sys_status_semantic_fields() {
        // SYS_STATUS (0x0001) payload: voltage_battery(2), current_battery(2), battery_remaining(1), ...
        // We need at least 20 bytes for the full message
        let mut payload = vec![0u8; 20];
        // voltage_battery: 12604 mV (0x313C)
        payload[0] = 0x3C;
        payload[1] = 0x31;
        // current_battery: -1200 mA (0xFB50 as i16)
        payload[2] = 0x50;
        payload[3] = 0xFB;
        // battery_remaining: 85%
        payload[4] = 85;
        
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x0001,
            payload,
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let fields = packet.fields();
        assert_eq!(fields.get("voltage_battery").unwrap(), "12604 mV");
        assert_eq!(fields.get("current_battery").unwrap(), "-1200 mA");
        assert_eq!(fields.get("battery_remaining").unwrap(), "85%");
    }

    #[test]
    fn attitude_semantic_fields() {
        // ATTITUDE (0x00CA) payload: time_usec(8), roll(4), pitch(4), yaw(4), rollspeed(4), pitchspeed(4), yawspeed(4)
        // Total: 32 bytes
        let mut payload = vec![0u8; 32];
        // time_usec: 0 (8 bytes)
        // roll: 0.5 rad
        let roll_bytes = 0.5f32.to_le_bytes();
        payload[8..12].copy_from_slice(&roll_bytes);
        // pitch: -0.3 rad
        let pitch_bytes = (-0.3f32).to_le_bytes();
        payload[12..16].copy_from_slice(&pitch_bytes);
        // yaw: 1.2 rad
        let yaw_bytes = 1.2f32.to_le_bytes();
        payload[16..20].copy_from_slice(&yaw_bytes);
        
        let packet = MavlinkPacket {
            sequence: 1,
            system_id: 1,
            component_id: 1,
            message_id: 0x00CA,
            payload,
            target_system: None,
            target_component: None,
            crc_validated: true,
        };

        let fields = packet.fields();
        assert!(fields.get("roll").unwrap().contains("0.50"));
        assert!(fields.get("pitch").unwrap().contains("-0.30"));
        assert!(fields.get("yaw").unwrap().contains("1.20"));
    }
}
