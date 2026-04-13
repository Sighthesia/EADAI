# Enhance MCP Numerical Analysis Capabilities

## 1. Purpose

Expose richer numerical analysis data through the MCP server to enable AI agents to perform quantitative analysis, trend detection, and parameter tuning without relying on image snapshots.

## 2. Background

The project already computes detailed channel analysis in the Rust backend:
- Time-domain metrics: `min_value`, `max_value`, `mean_value`, `rms_value`, `trend`, `change_rate`
- Frequency/duty metrics: `frequency_hz`, `period_ms`, `duty_cycle`
- Edge detection: `edge_count`, `rising_edge_count`, `falling_edge_count`
- Window metadata: `window_ms`, `sample_count`

These are exposed to the frontend via the bus, but the MCP server only exposes a summarized `analysis_frames` resource. The AI cannot yet retrieve detailed per-channel statistics or query historical windows.

## 3. Requirements

### 3.1 Richer Analysis Resource

Expose a more detailed analysis resource that includes:
- All computed metrics per channel (`frequency_hz`, `duty_cycle`, `trend`, `change_rate`, `rms`, etc.)
- Sample window metadata (window size, sample count, time span)
- Edge detection breakdown (rising/falling count, period stability)

### 3.2 New MCP Tool: get_channel_statistics

Add a new MCP tool that returns configurable time-window statistics for any channel:

```
get_channel_statistics({
  channel_id: string,      // Required: target channel
  window_ms: number,       // Optional: time window in ms (default: 1000)
  include_raw_samples: boolean  // Optional: include raw sample points
})
```

Returns:
- `channel_id`, `window_ms`, `sample_count`, `time_span_ms`
- `min`, `max`, `mean`, `rms`, `variance` (computed on demand)
- `trend`, `change_rate` (slope over window)
- `frequency_hz`, `period_ms`, `duty_cycle` (if periodic signal detected)

### 3.3 New MCP Tool: query_historical_analysis

Add a tool to query historical analysis frames (not just the latest):

```
query_historical_analysis({
  channel_id: string,
  start_time_ms: number,
  end_time_ms: number,
  max_frames: number   // Optional limit
})
```

Returns an array of analysis frames from the historical bounded buffer.

### 3.4 Edge Detection Details Resource

Expose edge detection breakdown as a separate resource or include in analysis frames:
- `rising_edges`, `falling_edges`, `total_edges`
- `period_ms` (average period)
- `duty_cycle` (high time percentage)

### 3.5 Copyable JSON in UI

Add a "Copy Analysis JSON" button to the Variables or MCP panel that copies the current analysis JSON to clipboard, allowing users to directly paste to AI prompts.

## 4. Architecture

```
AnalysisEngine (existing)
    │
    ├─ per-window frame computed on ingest
    └─ bounded history buffer (recent N frames)

AiContextAdapter (existing)
    │
    └─ exposes analysis_frames() → latest N frames

McpServer (to modify)
    │
    ├─ add new tool: get_channel_statistics()
    ├─ add new tool: query_historical_analysis()
    ├─ enhance analysis resource with edge details
    └─ expose variance (new computation)
```

## 5. Out of Scope

- Image snapshot rendering (`get_plot_snapshot` is a separate feature)
- Real-time streaming of analysis frames (resource pagination is sufficient)
- Complex statistical functions beyond variance (kurtosis, skewness)

## 6. Testing

- Add MCP integration tests for new tools
- Verify tool responses match expected JSON schema
- Verify null handling for channels with insufficient samples