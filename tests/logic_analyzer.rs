#[path = "../src-tauri/src/model/mod.rs"]
mod model;

#[path = "../src-tauri/src/logic_analyzer.rs"]
mod logic_analyzer;

use logic_analyzer::{
    LogicAnalyzerService, build_capture_command, parse_capture_csv_for_test, parse_scan_output,
};

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

#[test]
fn exposes_dev_simulator_and_generates_capture() {
    let service = LogicAnalyzerService::default();

    let status = service
        .refresh_devices()
        .expect("refresh logic analyzer devices");
    let simulator = status
        .devices
        .iter()
        .find(|device| device.reference == "dev://logic-analyzer")
        .expect("dev simulator should be present in debug builds");

    assert_eq!(simulator.name, "Logic Playground");
    assert_eq!(simulator.channels.len(), 8);

    let capture_status = service
        .start_capture(model::LogicAnalyzerCaptureRequest {
            device_ref: simulator.reference.clone(),
            sample_count: 32,
            samplerate_hz: None,
            channels: vec!["D0".to_string(), "D3".to_string(), "D7".to_string()],
        })
        .expect("start simulated capture");

    let capture = capture_status
        .last_capture
        .expect("simulated capture should be available immediately");
    assert_eq!(capture.sample_count, 32);
    assert_eq!(capture.channels.len(), 3);
    assert_eq!(capture.channels[0].label, "D0");
    assert_eq!(capture.channels[1].label, "D3");
    assert_eq!(capture.channels[2].label, "D7");
    assert!(capture.output_path.starts_with("dev://logic-capture/"));
}
