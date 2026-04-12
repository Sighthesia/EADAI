use eadai::ai_adapter::AiContextAdapter;
use eadai::mcp_server::TelemetryMcpServer;
use rmcp::ServerHandler;

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
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name.as_ref(), "get_channel_analysis");
    assert_eq!(tools[1].name.as_ref(), "get_recent_events");
    assert_eq!(
        tools[0].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
    assert_eq!(
        tools[1].annotations.as_ref().and_then(|a| a.read_only_hint),
        Some(true)
    );
}
