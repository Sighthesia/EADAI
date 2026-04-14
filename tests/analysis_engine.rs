use eadai::analysis::{AnalysisEngine, TriggerSeverity};
use eadai::message::{LineDirection, MessageKind, MessageSource, ParserMeta};
use std::collections::BTreeMap;

#[test]
fn pulse_stream_produces_waveform_metrics_and_edge_trigger() {
    let source = MessageSource::fake("fake://telemetry-lab", 115_200);
    let mut engine = AnalysisEngine::with_window_ms(2_000);
    let mut latest_frame = None;
    let mut fired_rules = Vec::new();

    for index in 0..25_u64 {
        let value = if index % 5 < 2 { 1.0 } else { 0.0 };
        if let Some(messages) = engine.ingest_line(
            &source,
            &LineDirection::Rx,
            &parser("pulse_signal", value, index * 100, true),
            index * 100,
        ) {
            capture_messages(&messages, &mut latest_frame, &mut fired_rules);
        }
    }

    let frame = latest_frame.expect("analysis frame for pulse stream");
    assert_eq!(frame.channel_id, "pulse_signal");
    assert!(frame.sample_count >= 8);
    assert!(frame.time_span_ms.expect("span") > 0.0);
    assert_approx(frame.frequency_hz.expect("frequency"), 2.0, 0.2);
    assert_approx(frame.period_ms.expect("period"), 500.0, 60.0);
    assert_approx(frame.duty_cycle.expect("duty"), 40.0, 8.0);
    assert!(frame.variance.expect("variance") >= 0.0);
    assert!(frame.period_stability.is_some());
    assert!(frame.edge_count >= 6);
    assert!(
        fired_rules
            .iter()
            .any(|(rule_id, _)| rule_id == "pulse-edge-burst")
    );
}

#[test]
fn noisy_stream_produces_rms_trigger_without_edge_metrics() {
    let source = MessageSource::fake("fake://noisy-monitor", 115_200);
    let mut engine = AnalysisEngine::with_window_ms(2_000);
    let mut latest_frame = None;
    let mut fired = Vec::new();
    let values = [0.52, 0.61, 0.49, 0.58, 0.54, 0.62, 0.47, 0.56, 0.53, 0.59];

    for (index, value) in values.into_iter().enumerate() {
        let timestamp_ms = index as u64 * 120;
        if let Some(messages) = engine.ingest_line(
            &source,
            &LineDirection::Rx,
            &parser("vibration_g", value, timestamp_ms, true),
            timestamp_ms,
        ) {
            capture_messages(&messages, &mut latest_frame, &mut fired);
        }
    }

    let frame = latest_frame.expect("analysis frame for noisy stream");
    assert_eq!(frame.channel_id, "vibration_g");
    assert!(frame.frequency_hz.is_none());
    assert!(frame.duty_cycle.is_none());
    assert!(frame.edge_count == 0);
    assert!(frame.rms_value.expect("rms") > 0.48);
    assert!(frame.variance.expect("variance") > 0.0);
    assert!(
        fired
            .iter()
            .any(|(rule_id, severity)| rule_id == "vibration-rms-high"
                && *severity == TriggerSeverity::Warning)
    );
}

#[test]
fn key_value_style_parser_without_numeric_field_is_accepted() {
    let source = MessageSource::fake("fake://telemetry-lab", 115_200);
    let mut engine = AnalysisEngine::with_window_ms(2_000);
    let mut latest_frame = None;
    let mut fired = Vec::new();

    for index in 0..4_u64 {
        let timestamp_ms = 500 + index * 100;
        let parser = parser("temp", 24.0 + index as f64, timestamp_ms, false);
        if let Some(messages) =
            engine.ingest_line(&source, &LineDirection::Rx, &parser, timestamp_ms)
        {
            capture_messages(&messages, &mut latest_frame, &mut fired);
        }
    }

    let frame = latest_frame.expect("analysis frame from value-only parser fields");
    assert_eq!(frame.channel_id, "temp");
    assert_eq!(frame.sample_count, 4);
    assert!(frame.mean_value.is_some());
    assert!(frame.time_span_ms.is_some());
}

#[test]
fn imu_raw_stream_produces_fused_attitude_lines() {
    let source = MessageSource::fake("fake://imu-lab", 115_200);
    let mut engine = AnalysisEngine::with_window_ms(2_000);
    let mut fused_lines = Vec::new();

    for index in 0..6_u64 {
        let timestamp_ms = index * 20;
        for (channel_id, value, unit) in [
            ("imu_ax", 0.0, "g"),
            ("imu_ay", 0.0, "g"),
            ("imu_az", 1.0, "g"),
            ("imu_gx", 0.0, "deg/s"),
            ("imu_gy", 0.0, "deg/s"),
            ("imu_gz", 12.0, "deg/s"),
        ] {
            if let Some(messages) = engine.ingest_line(
                &source,
                &LineDirection::Rx,
                &parser_with_unit(channel_id, value, unit, timestamp_ms, true),
                timestamp_ms,
            ) {
                capture_line_channels(&messages, &mut fused_lines);
            }
        }
    }

    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_roll"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_pitch"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_yaw"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_qw"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_qx"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_qy"));
    assert!(fused_lines.iter().any(|channel| channel == "imu_fused_qz"));
}

fn parser(channel_id: &str, value: f64, timestamp_ms: u64, include_numeric: bool) -> ParserMeta {
    parser_with_unit(channel_id, value, "", timestamp_ms, include_numeric)
}

fn parser_with_unit(
    channel_id: &str,
    value: f64,
    unit: &str,
    timestamp_ms: u64,
    include_numeric: bool,
) -> ParserMeta {
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    let display_value = if unit.is_empty() {
        value.to_string()
    } else {
        format!("{value}{unit}")
    };
    fields.insert("value".to_string(), display_value);
    if include_numeric {
        fields.insert("numeric_value".to_string(), value.to_string());
    }
    if !unit.is_empty() {
        fields.insert("unit".to_string(), unit.to_string());
    }
    fields.insert("timestamp".to_string(), timestamp_ms.to_string());
    ParserMeta::parsed("measurements", fields)
}

fn capture_messages(
    messages: &[eadai::message::BusMessage],
    latest_frame: &mut Option<eadai::analysis::AnalysisFrame>,
    fired: &mut Vec<(String, TriggerSeverity)>,
) {
    for message in messages {
        match &message.kind {
            MessageKind::Analysis(frame) => *latest_frame = Some(frame.clone()),
            MessageKind::Trigger(trigger) => {
                fired.push((trigger.rule_id.clone(), trigger.severity))
            }
            _ => {}
        }
    }
}

fn capture_line_channels(messages: &[eadai::message::BusMessage], fused_lines: &mut Vec<String>) {
    for message in messages {
        if let MessageKind::Line(_) = &message.kind
            && let Some(channel_id) = message.parser.fields.get("channel_id")
        {
            fused_lines.push(channel_id.clone());
        }
    }
}

fn assert_approx(actual: f64, expected: f64, tolerance: f64) {
    let delta = (actual - expected).abs();
    assert!(
        delta <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}, delta={delta}",
    );
}
