use crate::ai_adapter::AiContextAdapter;
use crate::ai_contract::{ChannelAnalysisQuery, RecentEventsQuery};
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde::Serialize;
use serde_json::{Value, json};

const SESSION_RESOURCE_URI: &str = "session://current";
const TELEMETRY_RESOURCE_URI: &str = "telemetry://summary";
const ANALYSIS_RESOURCE_URI: &str = "analysis://latest";
const TRIGGERS_RESOURCE_URI: &str = "triggers://recent";

/// MCP server that exposes the AI adapter over a read-only contract.
#[derive(Clone, Debug)]
pub struct TelemetryMcpServer {
    adapter: AiContextAdapter,
}

impl TelemetryMcpServer {
    /// Creates a read-only MCP server around the provided adapter.
    pub fn new(adapter: AiContextAdapter) -> Self {
        Self { adapter }
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
                tool_json_response(&resource)
            }
            "get_recent_events" => {
                let query: RecentEventsQuery = parse_arguments(request.arguments)?;
                let resource = self.adapter.recent_events(&query);
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
