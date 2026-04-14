use crate::message::{BusMessage, LinePayload, MessageSource, ParserMeta};
use crate::serial::payload_bytes_for_text;
use std::collections::BTreeMap;

const DEFAULT_SAMPLE_DT_S: f64 = 0.01;
const MAX_SAMPLE_DT_S: f64 = 0.2;
const MAHONY_KP: f64 = 1.0;
const MAHONY_KI: f64 = 0.05;
const IMU_FUSION_PARSER_NAME: &str = "imu-fusion";

#[derive(Clone, Debug, Default)]
pub(crate) struct ImuFusionEngine {
    sensors: BTreeMap<String, ImuSensorState>,
}

#[derive(Clone, Debug, Default)]
struct ImuSensorState {
    filter: MahonyFilter,
    pending: PendingSample,
    last_timestamp_ms: Option<u64>,
    output_prefix: String,
}

#[derive(Clone, Debug, Default)]
struct PendingSample {
    timestamp_ms: Option<u64>,
    accel: [Option<f64>; 3],
    gyro: [Option<f64>; 3],
}

#[derive(Clone, Copy, Debug)]
enum AxisKind {
    AccelX,
    AccelY,
    AccelZ,
    GyroX,
    GyroY,
    GyroZ,
}

#[derive(Clone, Copy, Debug)]
struct ImuSample {
    timestamp_ms: u64,
    accel: [f64; 3],
    gyro_rad_s: [f64; 3],
}

#[derive(Clone, Copy, Debug)]
struct OrientationEstimate {
    roll_deg: f64,
    pitch_deg: f64,
    yaw_deg: f64,
    quaternion: [f64; 4],
}

#[derive(Clone, Debug)]
struct MahonyFilter {
    quaternion: [f64; 4],
    integral_error: [f64; 3],
    initialized: bool,
}

impl ImuFusionEngine {
    pub(crate) fn ingest_measurement(
        &mut self,
        source: &MessageSource,
        parser: &ParserMeta,
        fallback_timestamp_ms: u64,
    ) -> Vec<BusMessage> {
        let Some(channel_id) = parser.fields.get("channel_id") else {
            return Vec::new();
        };
        let Some(value) = parser
            .fields
            .get("numeric_value")
            .or_else(|| parser.fields.get("value"))
            .and_then(|value| value.parse::<f64>().ok())
        else {
            return Vec::new();
        };
        let timestamp_ms = parser
            .fields
            .get("timestamp")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(fallback_timestamp_ms);
        let unit = parser.fields.get("unit").map(String::as_str);
        let Some((sensor_label, axis_kind)) = classify_raw_channel(channel_id) else {
            return Vec::new();
        };

        let sensor_key = format!("{}::{sensor_label}", source.port);
        let sensor = self.sensors.entry(sensor_key).or_default();
        sensor.output_prefix = sensor_label;

        let Some(sample) = sensor.pending.ingest(axis_kind, value, unit, timestamp_ms) else {
            return Vec::new();
        };

        let dt_s = sensor
            .last_timestamp_ms
            .and_then(|previous| timestamp_ms.checked_sub(previous))
            .map(|delta_ms| (delta_ms as f64 / 1000.0).clamp(0.0, MAX_SAMPLE_DT_S))
            .filter(|delta_s| *delta_s > 0.0)
            .unwrap_or(DEFAULT_SAMPLE_DT_S);
        sensor.last_timestamp_ms = Some(sample.timestamp_ms);

        let estimate = sensor.filter.update(sample, dt_s);
        build_fused_messages(source, &sensor.output_prefix, sample.timestamp_ms, estimate)
    }
}

impl PendingSample {
    fn ingest(
        &mut self,
        axis_kind: AxisKind,
        value: f64,
        unit: Option<&str>,
        timestamp_ms: u64,
    ) -> Option<ImuSample> {
        if self.timestamp_ms != Some(timestamp_ms) {
            self.timestamp_ms = Some(timestamp_ms);
            self.accel = [None, None, None];
            self.gyro = [None, None, None];
        }

        match axis_kind {
            AxisKind::AccelX => self.accel[0] = Some(value),
            AxisKind::AccelY => self.accel[1] = Some(value),
            AxisKind::AccelZ => self.accel[2] = Some(value),
            AxisKind::GyroX => self.gyro[0] = Some(normalize_gyro_unit(value, unit)),
            AxisKind::GyroY => self.gyro[1] = Some(normalize_gyro_unit(value, unit)),
            AxisKind::GyroZ => self.gyro[2] = Some(normalize_gyro_unit(value, unit)),
        }

        let [Some(ax), Some(ay), Some(az)] = self.accel else {
            return None;
        };
        let [Some(gx), Some(gy), Some(gz)] = self.gyro else {
            return None;
        };

        let sample = ImuSample {
            timestamp_ms,
            accel: [ax, ay, az],
            gyro_rad_s: [gx, gy, gz],
        };

        self.timestamp_ms = None;
        self.accel = [None, None, None];
        self.gyro = [None, None, None];
        Some(sample)
    }
}

impl Default for MahonyFilter {
    fn default() -> Self {
        Self {
            quaternion: [1.0, 0.0, 0.0, 0.0],
            integral_error: [0.0, 0.0, 0.0],
            initialized: false,
        }
    }
}

impl MahonyFilter {
    fn update(&mut self, sample: ImuSample, dt_s: f64) -> OrientationEstimate {
        if !self.initialized {
            self.quaternion = quaternion_from_accel(sample.accel);
            self.initialized = true;
            return self.orientation();
        }

        let accel_norm = magnitude(sample.accel);
        if accel_norm <= f64::EPSILON {
            return self.orientation();
        }
        let accel = [
            sample.accel[0] / accel_norm,
            sample.accel[1] / accel_norm,
            sample.accel[2] / accel_norm,
        ];

        let [q0, q1, q2, q3] = self.quaternion;
        let half_vx = q1 * q3 - q0 * q2;
        let half_vy = q0 * q1 + q2 * q3;
        let half_vz = q0 * q0 - 0.5 + q3 * q3;
        let error = [
            accel[1] * half_vz - accel[2] * half_vy,
            accel[2] * half_vx - accel[0] * half_vz,
            accel[0] * half_vy - accel[1] * half_vx,
        ];

        self.integral_error[0] += MAHONY_KI * error[0] * dt_s;
        self.integral_error[1] += MAHONY_KI * error[1] * dt_s;
        self.integral_error[2] += MAHONY_KI * error[2] * dt_s;

        let gx = sample.gyro_rad_s[0] + MAHONY_KP * error[0] + self.integral_error[0];
        let gy = sample.gyro_rad_s[1] + MAHONY_KP * error[1] + self.integral_error[1];
        let gz = sample.gyro_rad_s[2] + MAHONY_KP * error[2] + self.integral_error[2];
        let half_dt = 0.5 * dt_s;

        self.quaternion[0] += (-q1 * gx - q2 * gy - q3 * gz) * half_dt;
        self.quaternion[1] += (q0 * gx + q2 * gz - q3 * gy) * half_dt;
        self.quaternion[2] += (q0 * gy - q1 * gz + q3 * gx) * half_dt;
        self.quaternion[3] += (q0 * gz + q1 * gy - q2 * gx) * half_dt;
        normalize_quaternion(&mut self.quaternion);

        self.orientation()
    }

    fn orientation(&self) -> OrientationEstimate {
        let [q0, q1, q2, q3] = self.quaternion;
        let roll = (2.0 * (q0 * q1 + q2 * q3)).atan2(1.0 - 2.0 * (q1 * q1 + q2 * q2));
        let pitch = (2.0 * (q0 * q2 - q3 * q1)).clamp(-1.0, 1.0).asin();
        let yaw = (2.0 * (q0 * q3 + q1 * q2)).atan2(1.0 - 2.0 * (q2 * q2 + q3 * q3));

        OrientationEstimate {
            roll_deg: normalize_angle_deg(roll.to_degrees()),
            pitch_deg: normalize_angle_deg(pitch.to_degrees()),
            yaw_deg: normalize_angle_deg(yaw.to_degrees()),
            quaternion: self.quaternion,
        }
    }
}

fn classify_raw_channel(channel_id: &str) -> Option<(String, AxisKind)> {
    let normalized = normalize_channel_name(channel_id);
    let mut best_match = None;

    for (axis_kind, alias) in raw_aliases() {
        if normalized == *alias || normalized.ends_with(alias) {
            let prefix = normalized.strip_suffix(alias).unwrap_or_default();
            let score = alias.len();
            if best_match
                .as_ref()
                .map(|(best_score, _, _)| score > *best_score)
                .unwrap_or(true)
            {
                best_match = Some((score, sensor_label(prefix), *axis_kind));
            }
        }
    }

    best_match.map(|(_, label, axis_kind)| (label, axis_kind))
}

fn raw_aliases() -> &'static [(AxisKind, &'static str)] {
    &[
        (AxisKind::AccelX, "imuaccelx"),
        (AxisKind::AccelX, "imuaccx"),
        (AxisKind::AccelX, "imuax"),
        (AxisKind::AccelX, "accelx"),
        (AxisKind::AccelX, "accx"),
        (AxisKind::AccelX, "ax"),
        (AxisKind::AccelY, "imuaccely"),
        (AxisKind::AccelY, "imuaccy"),
        (AxisKind::AccelY, "imuay"),
        (AxisKind::AccelY, "accely"),
        (AxisKind::AccelY, "accy"),
        (AxisKind::AccelY, "ay"),
        (AxisKind::AccelZ, "imuaccelz"),
        (AxisKind::AccelZ, "imuaccz"),
        (AxisKind::AccelZ, "imuaz"),
        (AxisKind::AccelZ, "accelz"),
        (AxisKind::AccelZ, "accz"),
        (AxisKind::AccelZ, "az"),
        (AxisKind::GyroX, "imugyrox"),
        (AxisKind::GyroX, "imugx"),
        (AxisKind::GyroX, "gyrox"),
        (AxisKind::GyroX, "gx"),
        (AxisKind::GyroY, "imugyroy"),
        (AxisKind::GyroY, "imugy"),
        (AxisKind::GyroY, "gyroy"),
        (AxisKind::GyroY, "gy"),
        (AxisKind::GyroZ, "imugyroz"),
        (AxisKind::GyroZ, "imugz"),
        (AxisKind::GyroZ, "gyroz"),
        (AxisKind::GyroZ, "gz"),
    ]
}

fn sensor_label(prefix: &str) -> String {
    if prefix.is_empty() {
        return "imu".to_string();
    }

    if prefix.ends_with("imu") {
        return prefix.to_string();
    }

    format!("{prefix}_imu")
}

fn normalize_channel_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn normalize_gyro_unit(value: f64, unit: Option<&str>) -> f64 {
    let normalized_unit = unit.unwrap_or("deg/s").to_ascii_lowercase();
    if normalized_unit.contains("rad") {
        value
    } else {
        value.to_radians()
    }
}

fn build_fused_messages(
    source: &MessageSource,
    output_prefix: &str,
    timestamp_ms: u64,
    estimate: OrientationEstimate,
) -> Vec<BusMessage> {
    [
        ("roll", estimate.roll_deg),
        ("pitch", estimate.pitch_deg),
        ("yaw", estimate.yaw_deg),
    ]
    .into_iter()
    .map(|(axis_label, value)| {
        let channel_id = format!("{output_prefix}_fused_{axis_label}");
        let text = format!("timestamp={timestamp_ms} {channel_id}={value:.3}deg");
        let payload = LinePayload {
            text: text.clone(),
            raw: payload_bytes_for_text(&text, false),
        };

        BusMessage::rx_line(source, payload).with_parser(fused_parser(
            &channel_id,
            value,
            timestamp_ms,
        ))
    })
    .chain(
        [
            ("qw", estimate.quaternion[0]),
            ("qx", estimate.quaternion[1]),
            ("qy", estimate.quaternion[2]),
            ("qz", estimate.quaternion[3]),
        ]
        .into_iter()
        .map(|(axis_label, value)| {
            let channel_id = format!("{output_prefix}_fused_{axis_label}");
            let text = format!("timestamp={timestamp_ms} {channel_id}={value:.6}");
            let payload = LinePayload {
                text: text.clone(),
                raw: payload_bytes_for_text(&text, false),
            };

            BusMessage::rx_line(source, payload).with_parser(fused_parser_no_unit(
                &channel_id,
                value,
                timestamp_ms,
            ))
        }),
    )
    .collect()
}

fn fused_parser(channel_id: &str, value: f64, timestamp_ms: u64) -> ParserMeta {
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), format!("{value:.3}deg"));
    fields.insert("numeric_value".to_string(), format!("{value:.3}"));
    fields.insert("unit".to_string(), "deg".to_string());
    fields.insert("timestamp".to_string(), timestamp_ms.to_string());
    ParserMeta::parsed(IMU_FUSION_PARSER_NAME, fields)
}

fn fused_parser_no_unit(channel_id: &str, value: f64, timestamp_ms: u64) -> ParserMeta {
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), format!("{value:.6}"));
    fields.insert("numeric_value".to_string(), format!("{value:.6}"));
    fields.insert("timestamp".to_string(), timestamp_ms.to_string());
    ParserMeta::parsed(IMU_FUSION_PARSER_NAME, fields)
}

fn quaternion_from_accel(accel: [f64; 3]) -> [f64; 4] {
    let roll = accel[1].atan2(accel[2]);
    let pitch = (-accel[0]).atan2((accel[1] * accel[1] + accel[2] * accel[2]).sqrt());
    let (sr, cr) = (0.5 * roll).sin_cos();
    let (sp, cp) = (0.5 * pitch).sin_cos();

    let mut quaternion = [cr * cp, sr * cp, cr * sp, -sr * sp];
    normalize_quaternion(&mut quaternion);
    quaternion
}

fn magnitude(values: [f64; 3]) -> f64 {
    (values[0] * values[0] + values[1] * values[1] + values[2] * values[2]).sqrt()
}

fn normalize_quaternion(quaternion: &mut [f64; 4]) {
    let norm = (quaternion[0] * quaternion[0]
        + quaternion[1] * quaternion[1]
        + quaternion[2] * quaternion[2]
        + quaternion[3] * quaternion[3])
        .sqrt();
    if norm <= f64::EPSILON {
        *quaternion = [1.0, 0.0, 0.0, 0.0];
        return;
    }

    quaternion[0] /= norm;
    quaternion[1] /= norm;
    quaternion[2] /= norm;
    quaternion[3] /= norm;
}

fn normalize_angle_deg(value: f64) -> f64 {
    let mut normalized = value % 360.0;
    if normalized > 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}
