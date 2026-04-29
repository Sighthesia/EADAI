use eadai::ai_adapter::AiContextAdapter;
use eadai::analysis::{AnalysisFrame, TriggerEvent, TriggerSeverity};
use eadai::mcp_server::TelemetryMcpServer;
use eadai::mcp_server::McpToolUsageTracker;
use eadai::message::{BusMessage, MessageSource, ParserMeta, ParserStatus};
use rmcp::{
    ServerHandler,
    model::{CallToolRequestParam, NumberOrString, ReadResourceRequestParam, ResourceContents},
    service::{self, RequestContext, RoleServer},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio::io::DuplexStream;
use tokio_util::sync::CancellationToken;

#[test]
fn server_exposes_read_only_catalogs_and_capabilities() {
    let server = TelemetryMcpServer::new(AiContextAdapter::default());

    let info = server.get_info();
    assert!(
        info.instructions
            .as_deref()
            .is_some_and(|text| text.contains("Read-only telemetry MCP server"))
    );
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.tools.is_some());

    let resources = TelemetryMcpServer::resource_catalog();
    assert_eq!(resources.len(), 4);
    assert_eq!(resources[0].uri.as_str(), "session://current");
    assert_eq!(resources[1].uri.as_str(), "telemetry://summary");
    assert_eq!(resources[2].uri.as_str(), "analysis://latest");
    assert_eq!(resources[3].uri.as_str(), "triggers://recent");

    let tools = TelemetryMcpServer::tool_catalog();
    assert_eq!(tools.len(), 4);
    assert_eq!(tools[0].name.as_ref(), "get_channel_analysis");
    assert_eq!(tools[1].name.as_ref(), "get_recent_events");
    assert_eq!(tools[2].name.as_ref(), "get_channel_statistics");
    assert_eq!(tools[3].name.as_ref(), "query_historical_analysis");
    assert_eq!(
        tools[0].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
    assert_eq!(
        tools[1].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
    assert_eq!(
        tools[2].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
    assert_eq!(
        tools[3].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
}

#[tokio::test]
async fn reads_resources_and_calls_tools_over_mcp_transport() {
    let harness = spawn_server(populated_server());

    let telemetry = harness
        .running
        .service()
        .read_resource(
            ReadResourceRequestParam {
                uri: "telemetry://summary".to_string(),
            },
            harness.context(1),
        )
        .await
        .expect("read telemetry resource");
    let telemetry_json = parse_resource_json(&telemetry);
    assert_eq!(telemetry_json["channels"][0]["channel_id"], "temp");
    assert_eq!(telemetry_json["channels"][0]["trigger_count"], 1);

    let analysis = harness
        .running
        .service()
        .read_resource(
            ReadResourceRequestParam {
                uri: "analysis://latest".to_string(),
            },
            harness.context(2),
        )
        .await
        .expect("read analysis resource");
    let analysis_json = parse_resource_json(&analysis);
    assert_eq!(analysis_json["frames"][0]["channel_id"], "temp");
    assert!(analysis_json["frames"][0]["variance"].as_f64().is_some());

    let statistics = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "get_channel_statistics".into(),
                arguments: Some(
                    json!({
                        "channel_id": "temp",
                        "window_ms": 1_000,
                        "include_raw_samples": true
                    })
                    .as_object()
                    .expect("statistics arguments")
                    .clone(),
                ),
            },
            harness.context(3),
        )
        .await
        .expect("call statistics tool");
    let statistics_json: Value = statistics.into_typed().expect("statistics json");
    assert_eq!(statistics_json["channel_id"], "temp");
    assert_eq!(statistics_json["window_ms"], 1_000);
    assert!(statistics_json["sample_count"].as_u64().unwrap_or_default() >= 2);
    assert!(statistics_json["variance"].as_f64().is_some());
    assert!(statistics_json["raw_samples"].as_array().is_some());

    let historical = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "query_historical_analysis".into(),
                arguments: Some(
                    json!({
                        "channel_id": "temp",
                        "start_time_ms": 0,
                        "end_time_ms": u64::MAX,
                        "max_frames": 8
                    })
                    .as_object()
                    .expect("history arguments")
                    .clone(),
                ),
            },
            harness.context(4),
        )
        .await
        .expect("call history tool");
    let historical_json: Value = historical.into_typed().expect("history json");
    assert_eq!(historical_json["channel_id"], "temp");
    assert_eq!(historical_json["frames"].as_array().map(Vec::len), Some(2));

    harness.shutdown().await;
}

#[tokio::test]
async fn rejects_unknown_resources_and_invalid_tool_payloads() {
    let harness = spawn_server(populated_server());

    let unknown_resource = harness
        .running
        .service()
        .read_resource(
            ReadResourceRequestParam {
                uri: "analysis://missing".to_string(),
            },
            harness.context(10),
        )
        .await
        .expect_err("unknown resource should fail");
    let unknown_resource_text = unknown_resource.to_string();
    assert!(unknown_resource_text.contains("resource not found"));
    assert!(unknown_resource_text.contains("analysis://missing"));

    let unknown_tool = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "unknown_tool".into(),
                arguments: None,
            },
            harness.context(11),
        )
        .await
        .expect_err("unknown tool should fail");
    assert!(unknown_tool.to_string().contains("unknown tool"));

    let missing_channel = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "get_channel_statistics".into(),
                arguments: Some(
                    json!({
                        "channel_id": "missing"
                    })
                    .as_object()
                    .expect("missing channel args")
                    .clone(),
                ),
            },
            harness.context(12),
        )
        .await
        .expect_err("missing channel should fail");
    let missing_channel_text = missing_channel.to_string();
    assert!(missing_channel_text.contains("channel not found"));
    assert!(missing_channel_text.contains("missing"));

    let extra_field = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "get_recent_events".into(),
                arguments: Some(
                    json!({
                        "limit": 2,
                        "unexpected": true
                    })
                    .as_object()
                    .expect("extra field args")
                    .clone(),
                ),
            },
            harness.context(13),
        )
        .await
        .expect_err("extra tool field should fail");
    assert!(extra_field.to_string().contains("unknown field"));

    let invalid_enum = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "get_recent_events".into(),
                arguments: Some(
                    json!({
                        "kind": "not-a-kind"
                    })
                    .as_object()
                    .expect("invalid enum args")
                    .clone(),
                ),
            },
            harness.context(14),
        )
        .await
        .expect_err("invalid enum should fail");
    assert!(invalid_enum.to_string().contains("unknown variant"));

    harness.shutdown().await;
}

#[tokio::test]
async fn records_tool_usage_in_current_session() {
    let tracker = McpToolUsageTracker::new([
        "get_channel_analysis",
        "get_recent_events",
        "get_channel_statistics",
        "query_historical_analysis",
    ]);
    let server = TelemetryMcpServer::with_tool_usage(AiContextAdapter::default(), tracker.clone());
    let harness = spawn_server(server);

    let before = tracker.snapshot();
    assert!(before.iter().all(|entry| entry.last_called_at_ms.is_none()));

    let call = harness
        .running
        .service()
        .call_tool(
            CallToolRequestParam {
                name: "get_recent_events".into(),
                arguments: None,
            },
            harness.context(99),
        )
        .await;

    assert!(call.is_ok());

    let after = tracker.snapshot();
    let recent_events = after
        .iter()
        .find(|entry| entry.name == "get_recent_events")
        .expect("recent events usage entry");
    assert!(recent_events.last_called_at_ms.is_some());

    harness.shutdown().await;
}

struct ServerHarness {
    running: service::RunningService<RoleServer, TelemetryMcpServer>,
    _transport: DuplexStream,
}

impl ServerHarness {
    fn context(&self, request_id: i64) -> RequestContext<RoleServer> {
        RequestContext {
            peer: self.running.peer().clone(),
            ct: CancellationToken::new(),
            id: NumberOrString::Number(request_id),
            meta: Default::default(),
            extensions: Default::default(),
        }
    }

    async fn shutdown(self) {
        self.running.cancel().await.expect("cancel server");
    }
}

fn spawn_server(server: TelemetryMcpServer) -> ServerHarness {
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let running = service::serve_directly(server, server_transport, None);
    ServerHarness {
        running,
        _transport: client_transport,
    }
}

fn populated_server() -> TelemetryMcpServer {
    let adapter = AiContextAdapter::default();
    let source = MessageSource::fake("fake://telemetry-lab", 115_200);

    adapter.ingest(
        BusMessage::rx_line(&source, line_payload("temp=24.5")).with_parser(parser(
            "temp",
            "24.5",
            Some("24.5"),
            1_000,
        )),
    );
    adapter.ingest(
        BusMessage::rx_line(&source, line_payload("temp=25.0")).with_parser(parser(
            "temp",
            "25.0",
            Some("25.0"),
            1_400,
        )),
    );
    adapter.ingest(BusMessage::analysis(
        &source,
        AnalysisFrame {
            channel_id: "temp".to_string(),
            window_ms: 2_000,
            sample_count: 4,
            time_span_ms: Some(300.0),
            frequency_hz: Some(2.0),
            period_ms: Some(500.0),
            period_stability: Some(0.95),
            duty_cycle: Some(40.0),
            min_value: Some(24.0),
            max_value: Some(25.0),
            mean_value: Some(24.5),
            median_value: Some(24.5),
            rms_value: Some(24.52),
            variance: Some(0.05),
            edge_count: 6,
            rising_edge_count: 3,
            falling_edge_count: 3,
            trend: Some(0.5),
            change_rate: Some(1.25),
            trigger_hits: Vec::new(),
        },
    ));
    adapter.ingest(BusMessage::analysis(
        &source,
        AnalysisFrame {
            channel_id: "temp".to_string(),
            window_ms: 2_000,
            sample_count: 5,
            time_span_ms: Some(500.0),
            frequency_hz: Some(2.0),
            period_ms: Some(500.0),
            period_stability: Some(0.97),
            duty_cycle: Some(42.0),
            min_value: Some(24.1),
            max_value: Some(25.1),
            mean_value: Some(24.7),
            median_value: Some(24.7),
            rms_value: Some(24.73),
            variance: Some(0.04),
            edge_count: 6,
            rising_edge_count: 3,
            falling_edge_count: 3,
            trend: Some(0.6),
            change_rate: Some(1.2),
            trigger_hits: vec!["temp-high".to_string()],
        },
    ));
    adapter.ingest(BusMessage::trigger(
        &source,
        TriggerEvent {
            channel_id: "temp".to_string(),
            rule_id: "temp-high".to_string(),
            severity: TriggerSeverity::Warning,
            fired_at_ms: 1_500,
            reason: "temperature exceeded threshold".to_string(),
            snapshot: None,
        },
    ));

    TelemetryMcpServer::new(adapter)
}

fn parse_resource_json(resource: &rmcp::model::ReadResourceResult) -> Value {
    let ResourceContents::TextResourceContents {
        mime_type, text, ..
    } = &resource.contents[0]
    else {
        panic!("expected text resource");
    };
    assert_eq!(mime_type.as_deref(), Some("application/json"));
    serde_json::from_str(text).expect("resource json")
}

fn parser(
    channel_id: &str,
    value: &str,
    numeric_value: Option<&str>,
    timestamp_ms: u64,
) -> ParserMeta {
    let mut fields = BTreeMap::new();
    fields.insert("channel_id".to_string(), channel_id.to_string());
    fields.insert("value".to_string(), value.to_string());
    if let Some(numeric_value) = numeric_value {
        fields.insert("numeric_value".to_string(), numeric_value.to_string());
    }
    fields.insert("timestamp".to_string(), timestamp_ms.to_string());

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
