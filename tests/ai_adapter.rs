use eadai::ai_adapter::AiContextAdapter;
use eadai::ai_contract::{AiRecentEventKind, RecentEventsQuery};
use eadai::analysis::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use eadai::message::{BusMessage, ConnectionState, MessageSource, ParserMeta, ParserStatus};
use std::collections::BTreeMap;

#[test]
fn adapter_builds_session_summary_analysis_and_trigger_history() {
    let adapter = AiContextAdapter::default();
    let source = MessageSource::fake("fake://telemetry-lab", 115_200);

    adapter.ingest(BusMessage::connection(
        &source,
        ConnectionState::Connected,
        None,
        1,
        None,
    ));
    adapter.ingest(
        BusMessage::rx_line(&source, line_payload("temp=24.5")).with_parser(parser(
            "temp",
            "24.5",
            Some("24.5"),
        )),
    );
    adapter.ingest(BusMessage::analysis(
        &source,
        AnalysisFrame {
            channel_id: "temp".to_string(),
            window_ms: 2_000,
            sample_count: 8,
            frequency_hz: None,
            period_ms: None,
            duty_cycle: None,
            min_value: Some(23.8),
            max_value: Some(24.8),
            mean_value: Some(24.3),
            rms_value: Some(24.31),
            edge_count: 0,
            rising_edge_count: 0,
            falling_edge_count: 0,
            trend: Some(0.2),
            change_rate: Some(0.1),
            trigger_hits: vec!["temp-high".to_string()],
        },
    ));
    adapter.ingest(BusMessage::trigger(
        &source,
        TriggerEvent {
            channel_id: "temp".to_string(),
            rule_id: "temp-high".to_string(),
            severity: TriggerSeverity::Warning,
            fired_at_ms: 1_200,
            reason: "temperature exceeded threshold".to_string(),
            snapshot: None,
        },
    ));

    let session = adapter.session_snapshot();
    assert!(session.is_running);
    assert_eq!(session.source, Some(source.clone()));
    assert_eq!(
        session.connection.expect("connection").state,
        ConnectionState::Connected
    );

    let telemetry = adapter.telemetry_summary();
    assert_eq!(telemetry.channels.len(), 1);
    assert_eq!(telemetry.channels[0].channel_id, "temp");
    assert_eq!(telemetry.channels[0].current_value.as_deref(), Some("24.5"));
    assert_eq!(telemetry.channels[0].numeric_value, Some(24.5));
    assert!(telemetry.channels[0].has_analysis);
    assert_eq!(telemetry.channels[0].trigger_count, 1);

    let analysis = adapter.analysis_frames();
    assert_eq!(analysis.frames.len(), 1);
    assert_eq!(analysis.frames[0].channel_id, "temp");

    let triggers = adapter.trigger_history();
    assert_eq!(triggers.triggers.len(), 1);
    assert_eq!(triggers.triggers[0].rule_id, "temp-high");

    let channel = adapter
        .channel_analysis("temp", true)
        .expect("channel analysis");
    assert_eq!(channel.channel_id, "temp");
    assert_eq!(channel.recent_triggers.len(), 1);
}

#[test]
fn adapter_filters_recent_events_by_kind_and_channel() {
    let adapter = AiContextAdapter::default();
    let source = MessageSource::fake("fake://telemetry-lab", 115_200);

    adapter.ingest(BusMessage::connection(
        &source,
        ConnectionState::Connected,
        None,
        1,
        None,
    ));
    adapter.ingest(
        BusMessage::rx_line(&source, line_payload("temp=24.5")).with_parser(parser(
            "temp",
            "24.5",
            Some("24.5"),
        )),
    );
    adapter.ingest(BusMessage::analysis(
        &source,
        AnalysisFrame {
            channel_id: "temp".to_string(),
            window_ms: 2_000,
            sample_count: 4,
            frequency_hz: None,
            period_ms: None,
            duty_cycle: None,
            min_value: Some(24.0),
            max_value: Some(25.0),
            mean_value: Some(24.5),
            rms_value: Some(24.52),
            edge_count: 0,
            rising_edge_count: 0,
            falling_edge_count: 0,
            trend: None,
            change_rate: None,
            trigger_hits: Vec::new(),
        },
    ));
    adapter.ingest(BusMessage::analysis(
        &source,
        AnalysisFrame {
            channel_id: "pressure".to_string(),
            window_ms: 2_000,
            sample_count: 4,
            frequency_hz: None,
            period_ms: None,
            duty_cycle: None,
            min_value: Some(101.0),
            max_value: Some(102.0),
            mean_value: Some(101.4),
            rms_value: Some(101.41),
            edge_count: 0,
            rising_edge_count: 0,
            falling_edge_count: 0,
            trend: None,
            change_rate: None,
            trigger_hits: Vec::new(),
        },
    ));

    let filtered = adapter.recent_events(&RecentEventsQuery {
        limit: Some(8),
        kind: Some(AiRecentEventKind::Analysis),
        channel_id: Some("temp".to_string()),
    });

    assert_eq!(filtered.events.len(), 1);
    assert_eq!(filtered.events[0].kind, AiRecentEventKind::Analysis);
    assert_eq!(
        filtered.events[0]
            .analysis
            .as_ref()
            .map(|frame| frame.channel_id.as_str()),
        Some("temp")
    );
}

fn parser(channel_id: &str, value: &str, numeric_value: Option<&str>) -> ParserMeta {
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), value.to_string());
    if let Some(numeric_value) = numeric_value {
        fields.insert("numeric_value".to_string(), numeric_value.to_string());
    }

    ParserMeta {
        parser_name: Some("measurements".to_string()),
        status: ParserStatus::Parsed,
        fields,
    }
}

fn line_payload(text: &str) -> eadai::message::LinePayload {
    eadai::message::LinePayload {
        text: text.to_string(),
        raw: text.as_bytes().to_vec(),
    }
}
