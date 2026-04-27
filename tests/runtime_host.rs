use eadai::fake_session;
use eadai::message::{MessageKind, TransportKind};
use eadai::runtime_host::{FakeRuntimeConfig, RuntimeSessionConfig, SessionRuntimeHost};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn fake_session_feeds_subscription_and_shared_adapter() {
    let host = SessionRuntimeHost::default();
    let subscription = host
        .connect(RuntimeSessionConfig::Fake(FakeRuntimeConfig {
            profile: fake_session::default_profile().to_string(),
            baud_rate: 115200,
        }))
        .expect("start fake runtime host");

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut saw_telemetry_message = false;

    while Instant::now() < deadline {
        if let Ok(message) = subscription.recv_timeout(Duration::from_millis(100))
            && matches!(
                message.kind,
                MessageKind::Line(_)
                    | MessageKind::ShellOutput(_)
                    | MessageKind::TelemetrySchema(_)
                    | MessageKind::TelemetrySample(_)
                    | MessageKind::Analysis(_)
                    | MessageKind::Trigger(_)
            )
        {
            saw_telemetry_message = true;
            break;
        }
    }

    let telemetry_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < telemetry_deadline
        && host.adapter().telemetry_summary().channels.is_empty()
    {
        thread::sleep(Duration::from_millis(25));
    }

    let snapshot = host.adapter().session_snapshot();
    assert!(
        saw_telemetry_message,
        "expected fake runtime to publish telemetry events"
    );
    assert!(
        snapshot.is_running,
        "expected shared adapter session to be running"
    );
    assert_eq!(
        snapshot.source.expect("session source").transport,
        TransportKind::Fake
    );
    assert!(
        !host.adapter().telemetry_summary().channels.is_empty(),
        "expected shared adapter telemetry summary to populate"
    );

    host.disconnect().expect("disconnect fake runtime host");
}

#[test]
fn disconnect_resets_shared_adapter_snapshots() {
    let host = SessionRuntimeHost::default();
    host.connect(RuntimeSessionConfig::Fake(FakeRuntimeConfig {
        profile: fake_session::default_profile().to_string(),
        baud_rate: 115200,
    }))
    .expect("start fake runtime host");

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && host.adapter().telemetry_summary().channels.is_empty() {
        thread::sleep(Duration::from_millis(25));
    }

    host.disconnect().expect("disconnect fake runtime host");

    let snapshot = host.adapter().session_snapshot();
    assert!(!snapshot.is_running);
    assert!(snapshot.source.is_none());
    assert!(snapshot.connection.is_none());
    assert!(host.adapter().telemetry_summary().channels.is_empty());
    assert!(host.adapter().analysis_frames().frames.is_empty());
    assert!(host.adapter().trigger_history().triggers.is_empty());
}

#[test]
fn imu_fake_profile_emits_fused_attitude_lines() {
    let host = SessionRuntimeHost::default();
    let subscription = host
        .connect(RuntimeSessionConfig::Fake(FakeRuntimeConfig {
            profile: "imu-lab".to_string(),
            baud_rate: 115200,
        }))
        .expect("start imu fake runtime host");

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut saw_fused_roll = false;
    let mut saw_fused_qw = false;

    while Instant::now() < deadline {
        if let Ok(message) = subscription.recv_timeout(Duration::from_millis(100))
            && let MessageKind::Line(_) = &message.kind
        {
            match message.parser.fields.get("channel_id").map(String::as_str) {
                Some("imu_fused_roll") => saw_fused_roll = true,
                Some("imu_fused_qw") => saw_fused_qw = true,
                _ => {}
            }
            if saw_fused_roll && saw_fused_qw {
                break;
            }
        }
    }

    assert!(
        saw_fused_roll,
        "expected imu fake runtime to publish fused attitude lines"
    );
    assert!(
        saw_fused_qw,
        "expected imu fake runtime to publish fused quaternion lines"
    );

    host.disconnect().expect("disconnect imu fake runtime host");
}
