use crate::ai_adapter::AiContextAdapter;
use crate::ai_contract::{
    ChannelAnalysisQuery, ChannelStatisticsQuery, HistoricalAnalysisQuery, RecentEventsQuery,
};
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

const SESSION_RESOURCE_URI: &str = "session://current";
const TELEMETRY_RESOURCE_URI: &str = "telemetry://summary";
const ANALYSIS_RESOURCE_URI: &str = "analysis://latest";
const TRIGGERS_RESOURCE_URI: &str = "triggers://recent";

/// MCP server that exposes the AI adapter over a read-only contract.
#[derive(Clone, Debug)]
pub struct TelemetryMcpServer {
    adapter: AiContextAdapter,
    tool_usage: McpToolUsageTracker,
}

impl TelemetryMcpServer {
    /// Creates a read-only MCP server around the provided adapter.
    pub fn new(adapter: AiContextAdapter) -> Self {
        Self {
            adapter,
            tool_usage: McpToolUsageTracker::new(Self::tool_names()),
        }
    }

    /// Creates a read-only MCP server around the provided adapter and tracker.
    pub fn with_tool_usage(adapter: AiContextAdapter, tool_usage: McpToolUsageTracker) -> Self {
        Self { adapter, tool_usage }
    }

    /// Returns the static resource catalog exposed to MCP clients.
    pub fn resource_catalog() -> Vec<Resource> {
        vec![
            resource(
                SESSION_RESOURCE_URI,
                "Current Session",
                "Current runtime connection state",
            ),
            resource(
                TELEMETRY_RESOURCE_URI,
                "Telemetry Summary",
                "Latest telemetry summary per channel",
            ),
            resource(
                ANALYSIS_RESOURCE_URI,
                "Latest Analysis",
                "Latest analysis frame per channel",
            ),
            resource(
                TRIGGERS_RESOURCE_URI,
                "Recent Triggers",
                "Bounded recent trigger history",
            ),
        ]
    }

    /// Returns the static tool catalog exposed to MCP clients.
    pub fn tool_catalog() -> Vec<Tool> {
        vec![
            Tool::new(
                "get_channel_analysis",
                "Return telemetry, latest analysis, and optional trigger context for one channel",
                json_object(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "string", "description": "Target channel id" },
                        "include_trigger_context": { "type": "boolean", "description": "Include recent trigger history for the channel" }
                    },
                    "required": ["channel_id"],
                    "additionalProperties": false
                })),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            Tool::new(
                "get_recent_events",
                "Return a bounded filtered list of recent bus events",
                json_object(json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "description": "Maximum number of events to return" },
                        "kind": { "type": "string", "enum": ["connection", "line", "analysis", "trigger"], "description": "Optional event kind filter" },
                        "channel_id": { "type": "string", "description": "Optional channel filter for analysis/trigger events" }
                    },
                    "additionalProperties": false
                })),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            Tool::new(
                "get_channel_statistics",
                "Return sampled channel statistics for a time window",
                json_object(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "string", "description": "Target channel id" },
                        "window_ms": { "type": "integer", "minimum": 1, "default": 1000, "description": "Optional sample window in ms (defaults to 1000 ms)" },
                        "include_raw_samples": { "type": "boolean", "description": "Include raw bounded sample points" }
                    },
                    "required": ["channel_id"],
                    "additionalProperties": false
                })),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
            Tool::new(
                "query_historical_analysis",
                "Return historical analysis frames for a channel and time range",
                json_object(json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "string", "description": "Target channel id" },
                        "start_time_ms": { "type": "integer", "minimum": 0, "description": "Inclusive start time" },
                        "end_time_ms": { "type": "integer", "minimum": 0, "description": "Inclusive end time" },
                        "max_frames": { "type": "integer", "minimum": 1, "description": "Optional frame limit" }
                    },
                    "required": ["channel_id", "start_time_ms", "end_time_ms"],
                    "additionalProperties": false
                })),
            )
            .annotate(ToolAnnotations::new().read_only(true)),
        ]
    }

    /// Returns the current in-session tool usage snapshot.
    pub fn tool_usage_snapshot(&self) -> Vec<McpToolUsageSnapshot> {
        self.tool_usage.snapshot()
    }

    fn tool_names() -> [&'static str; 4] {
        [
            "get_channel_analysis",
            "get_recent_events",
            "get_channel_statistics",
            "query_historical_analysis",
        ]
    }
}

impl ServerHandler for TelemetryMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Read-only telemetry MCP server. Resources expose current session, telemetry summaries, latest analysis frames, and recent triggers. Tools support filtered recent-event inspection and per-channel analysis lookups.".to_string(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _cx: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: Self::resource_catalog(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _cx: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = request.uri.as_str();
        let json_text = match uri {
            SESSION_RESOURCE_URI => encode_json(&self.adapter.session_snapshot())?,
            TELEMETRY_RESOURCE_URI => encode_json(&self.adapter.telemetry_summary())?,
            ANALYSIS_RESOURCE_URI => encode_json(&self.adapter.analysis_frames())?,
            TRIGGERS_RESOURCE_URI => encode_json(&self.adapter.trigger_history())?,
            _ => {
                return Err(ErrorData::resource_not_found(
                    "resource not found",
                    Some(json!({ "uri": uri })),
                ));
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::TextResourceContents {
                uri: uri.into(),
                mime_type: Some("application/json".into()),
                text: json_text,
                meta: None,
            }],
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _cx: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
            meta: None,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _cx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: Self::tool_catalog(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _cx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        match request.name.as_ref() {
            "get_channel_analysis" => {
                let query: ChannelAnalysisQuery = parse_arguments(request.arguments)?;
                let Some(resource) = self
                    .adapter
                    .channel_analysis(&query.channel_id, query.include_trigger_context)
                else {
                    return Err(ErrorData::resource_not_found(
                        "channel not found",
                        Some(json!({ "channel_id": query.channel_id })),
                    ));
                };
                self.tool_usage.record("get_channel_analysis");
                tool_json_response(&resource)
            }
            "get_recent_events" => {
                let query: RecentEventsQuery = parse_arguments(request.arguments)?;
                let resource = self.adapter.recent_events(&query);
                self.tool_usage.record("get_recent_events");
                tool_json_response(&resource)
            }
            "get_channel_statistics" => {
                let query: ChannelStatisticsQuery = parse_arguments(request.arguments)?;
                let Some(resource) = self.adapter.channel_statistics(&query) else {
                    return Err(ErrorData::resource_not_found(
                        "channel not found",
                        Some(json!({ "channel_id": query.channel_id })),
                    ));
                };
                self.tool_usage.record("get_channel_statistics");
                tool_json_response(&resource)
            }
            "query_historical_analysis" => {
                let query: HistoricalAnalysisQuery = parse_arguments(request.arguments)?;
                let Some(resource) = self.adapter.historical_analysis(&query) else {
                    return Err(ErrorData::resource_not_found(
                        "channel not found",
                        Some(json!({ "channel_id": query.channel_id })),
                    ));
                };
                self.tool_usage.record("query_historical_analysis");
                tool_json_response(&resource)
            }
            _ => Err(ErrorData::invalid_params(
                format!("unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

fn resource(uri: &str, name: &str, description: &str) -> Resource {
    RawResource {
        uri: uri.into(),
        name: name.into(),
        title: None,
        description: Some(description.into()),
        mime_type: Some("application/json".into()),
        size: None,
        icons: None,
        meta: None,
    }
    .no_annotation()
}

fn tool_json_response<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(encode_json(
        value,
    )?)]))
}

fn encode_json<T: Serialize>(value: &T) -> Result<String, ErrorData> {
    serde_json::to_string_pretty(value)
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn parse_arguments<T: serde::de::DeserializeOwned>(
    arguments: Option<JsonObject>,
) -> Result<T, ErrorData> {
    let value = Value::Object(arguments.unwrap_or_default());
    serde_json::from_value(value)
        .map_err(|error| ErrorData::invalid_params(error.to_string(), None))
}

fn json_object(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => panic!("expected JSON object"),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolUsageSnapshot {
    pub name: String,
    pub last_called_at_ms: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct McpToolUsageTracker {
    inner: Arc<Mutex<Vec<McpToolUsageSnapshot>>>,
}

impl McpToolUsageTracker {
    pub fn new<I, S>(tool_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let inner = tool_names
            .into_iter()
            .map(|name| McpToolUsageSnapshot {
                name: name.into(),
                last_called_at_ms: None,
            })
            .collect();

        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn record(&self, tool_name: &str) {
        let mut usage = lock_tool_usage(&self.inner);
        if let Some(entry) = usage.iter_mut().find(|entry| entry.name == tool_name) {
            entry.last_called_at_ms = Some(timestamp_ms(SystemTime::now()));
        }
    }

    pub fn snapshot(&self) -> Vec<McpToolUsageSnapshot> {
        lock_tool_usage(&self.inner).clone()
    }
}

fn timestamp_ms(timestamp: SystemTime) -> u64 {
    timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn lock_tool_usage<'a>(usage: &'a Arc<Mutex<Vec<McpToolUsageSnapshot>>>) -> MutexGuard<'a, Vec<McpToolUsageSnapshot>> {
    match usage.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
