#[path = "../src-tauri/src/model.rs"]
mod model;

#[path = "../src-tauri/src/logic_analyzer.rs"]
mod logic_analyzer;

use logic_analyzer::{build_capture_command, parse_capture_csv_for_test, parse_scan_output};

#[test]
fn parses_sigrok_scan_output_into_devices() {
    let output = "Found 1 device(s).\nfx2lafw [Demo Logic Analyzer]\n";

    let devices = parse_scan_output(output);

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].reference, "Demo Logic Analyzer");
    assert_eq!(devices[0].name, "fx2lafw");
}

#[test]
fn builds_capture_command_with_optional_samplerate() {
    let request = model::LogicAnalyzerCaptureRequest {
        device_ref: "demo:conn=1".to_string(),
        sample_count: 2048,
        samplerate_hz: Some(1_000_000),
        channels: vec!["D0".to_string(), "D1".to_string()],
    };

    let command = build_capture_command("sigrok-cli", &request, "/tmp/capture.csv");

    assert!(command.contains("sigrok-cli"));
    assert!(command.contains("-d 'demo:conn=1'"));
    assert!(command.contains("--samples 2048"));
    assert!(command.contains("samplerate=1000000"));
    assert!(command.contains("-C 'D0,D1'"));
    assert!(command.contains("-o /tmp/capture.csv"));
}

#[test]
fn parses_simple_csv_waveform_into_channels() {
    let csv = "time,D0,D1\n0,0,1\n1,1,0\n2,1,1\n";

    let parsed = parse_capture_csv_for_test(csv, 3);

    assert_eq!(parsed.sample_count, 3);
    assert_eq!(parsed.channels.len(), 2);
    assert_eq!(parsed.channels[0].label, "D0");
    assert_eq!(
        parsed.channels[0].samples,
        vec![Some(false), Some(true), Some(true)]
    );
    assert_eq!(parsed.channels[1].label, "D1");
    assert_eq!(
        parsed.channels[1].samples,
        vec![Some(true), Some(false), Some(true)]
    );
}
