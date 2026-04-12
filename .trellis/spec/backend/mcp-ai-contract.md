# MCP AI Contract

> Executable reference for the read-only MCP boundary that exposes telemetry state to AI clients.

---

## Boundary Overview

The MCP layer is intentionally thin:

```text
BusMessage stream -> AiContextAdapter -> TelemetryMcpServer -> MCP client
                     ^
                     |
             shared desktop runtime
```

Files that define the boundary:

- `src/ai_adapter.rs` collects live bus messages into bounded snapshots.
- `src/ai_contract.rs` defines the AI-facing resource and tool payload shapes.
- `src/mcp_server.rs` exposes those payloads as read-only MCP resources and tools.
- `src/runtime_host.rs` owns the single live runtime session that can now be shared by the desktop UI and embedded MCP server.
- `tests/ai_adapter.rs` verifies the snapshot contract.
- `tests/mcp_server.rs` verifies the MCP catalog and read-only hints.

In desktop mode, the embedded MCP server must read the same `AiContextAdapter` instance as the visible UI session. Fake and serial source switches should therefore affect UI and MCP together.

This boundary must stay read-only in the first version. Do not let MCP handlers call serial write paths, UI state stores, or transport internals directly.

---

## Resource Contract

Declared in `src/mcp_server.rs` via `TelemetryMcpServer::resource_catalog()`.

| URI | Name | Source | Response shape |
|-----|------|--------|----------------|
| `session://current` | Current Session | `AiContextAdapter::session_snapshot()` | `AiSessionSnapshot` |
| `telemetry://summary` | Telemetry Summary | `AiContextAdapter::telemetry_summary()` | `TelemetrySummaryResource` |
| `analysis://latest` | Latest Analysis | `AiContextAdapter::analysis_frames()` | `AnalysisFramesResource` |
| `triggers://recent` | Recent Triggers | `AiContextAdapter::trigger_history()` | `TriggerHistoryResource` |

All resources must return JSON text with `mime_type: application/json`.

### Required Payload Fields

- `AiSessionSnapshot`
  - `is_running`
  - `source`
  - `connection`
  - `last_event_at_ms`
- `TelemetrySummaryResource`
  - `channels[]`
  - `channels[].channel_id`
  - `channels[].current_value`
  - `channels[].numeric_value`
  - `channels[].parser_name`
  - `channels[].updated_at_ms`
  - `channels[].has_analysis`
  - `channels[].trigger_count`
  - `channels[].latest_trigger_severity`
  - `channels[].latest_trigger_reason`
- `AnalysisFramesResource`
  - `frames[]` from the existing `AnalysisFrame` model
- `TriggerHistoryResource`
  - `triggers[]` from the existing `TriggerEvent` model

---

## Tool Contract

Declared in `src/mcp_server.rs` via `TelemetryMcpServer::tool_catalog()`.

### `get_channel_analysis`

- Handler: `TelemetryMcpServer::call_tool`
- Query type: `ChannelAnalysisQuery`
- Required input fields:
  - `channel_id: string`
- Optional input fields:
  - `include_trigger_context: boolean`
- Output type: `ChannelAnalysisResource`
- Output fields:
  - `channel_id`
  - `telemetry`
  - `analysis`
  - `recent_triggers[]`

### `get_recent_events`

- Handler: `TelemetryMcpServer::call_tool`
- Query type: `RecentEventsQuery`
- Optional input fields:
  - `limit: integer >= 1`
  - `kind: connection | line | analysis | trigger`
  - `channel_id: string`
- Output type: `RecentEventsResource`
- Output fields:
  - `events[]`
  - `events[].timestamp_ms`
  - `events[].source`
  - `events[].kind`
  - `events[].connection`
  - `events[].line`
  - `events[].analysis`
  - `events[].trigger`

All tools must advertise `read_only_hint = true`.

---

## Validation Rules

- Validate tool arguments at the MCP entry point with `parse_arguments()`.
- Reject extra tool fields with `additionalProperties: false` in the tool schema.
- Treat unknown resource URIs as MCP `resource_not_found` errors.
- Treat unknown tool names or invalid argument payloads as MCP `invalid_params` errors.
- Keep bounded history in `AiContextAdapter` so the MCP layer cannot expose unbounded event growth.

---

## Error Matrix

| Boundary | Condition | Expected result |
|----------|-----------|-----------------|
| `read_resource` | Unknown URI | `ErrorData::resource_not_found` with `uri` in `data` |
| `call_tool` | Unknown tool name | `ErrorData::invalid_params` |
| `call_tool` | Bad JSON shape or bad enum value | `ErrorData::invalid_params` |
| `call_tool` | Channel missing for `get_channel_analysis` | `ErrorData::resource_not_found` with `channel_id` in `data` |
| `encode_json` | Serialization failure | `ErrorData::internal_error` |

Do not return ad-hoc strings or custom one-off error payloads from MCP handlers.

---

## Good / Base / Bad Cases

### Good

- A client reads `telemetry://summary` and gets parsed channel summaries without touching raw serial text.
- A client calls `get_recent_events` with `{ "kind": "trigger", "limit": 5 }` and receives only trigger events.
- A client calls `get_channel_analysis` with `include_trigger_context = true` and receives bounded recent trigger context for that channel.

### Base

- A fresh adapter with no traffic still exposes all resources and tools in the catalog.
- `get_recent_events` with no arguments returns the default bounded event view.
- `get_channel_analysis` can return telemetry only, analysis only, or both, depending on what the adapter has observed.

### Bad

- Adding write tools that forward commands to serial transport from this MCP server.
- Returning UI-specific store objects instead of `src/ai_contract.rs` types.
- Recomputing analysis in the MCP handler instead of reusing adapter snapshots.
- Exposing raw unbounded serial history through a resource or tool.

---

## Required Tests

When changing `src/ai_adapter.rs`, `src/ai_contract.rs`, or `src/mcp_server.rs`, keep these assertions covered:

- `tests/ai_adapter.rs`
  - Session snapshot reflects connection state.
  - Telemetry summaries expose parsed values and analysis presence.
  - Trigger history stays structured and channel-aware.
  - Recent event filtering by kind and channel keeps the contract stable.
- `tests/mcp_server.rs`
  - `get_info()` still advertises resources and tools.
  - Resource URIs stay stable.
  - Tool names stay stable.
  - Every tool stays marked read-only.

Add or extend tests if you change URI names, tool names, payload shapes, or error behavior.
