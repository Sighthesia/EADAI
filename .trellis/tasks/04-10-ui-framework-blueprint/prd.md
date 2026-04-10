# UI Framework Blueprint for Telemetry Visualization

## 1. Purpose

This blueprint defines a **MVP-first** UI architecture for the telemetry visualization workstation described in `ROADMAP.md`.
It is intended to be **directionally strong but not over-constraining**: teams should be able to implement the core path directly, while still retaining room to swap charting strategy, refine IPC, or deepen Rust-native experimentation later.

## 2. Design Position

### Recommended baseline

- Host shell: `Tauri`
- UI framework: `React`
- Realtime waveform rendering: `uPlot`
- 3D visualization: `Three.js`
- Backend runtime and data pipeline: `Rust`

### Why this baseline

- Matches the architecture already implied by `ROADMAP.md`
- Keeps Rust on the critical path for I/O and data processing
- Uses the strongest available chart and 3D ecosystem for the UI layer
- Preserves a clean bridge for AI-facing features like MCP control and plot snapshots

### Freedom left intentionally open

- `React` may later be replaced by `Vue` if the team prefers a simpler component model
- `uPlot` may coexist with `ECharts` rather than fully replace it
- Tauri event/channel strategy may evolve after real throughput measurements
- A Rust-native prototype path using `egui` may be explored as a benchmarking branch, not the default product path

## 3. Product Priorities

This blueprint optimizes for the following order:

1. High-frequency telemetry responsiveness
2. Reliable waveform readability and interaction
3. AI collaboration and multimodal integration
4. Fast MVP delivery with low architecture regret

## 4. Non-Goals for MVP

- No attempt to build a generalized plugin marketplace
- No requirement to support every transport beyond serial in the first UI iteration
- No commitment to a full docking layout system on day one
- No requirement to make every graphing surface fully configurable before validating the core workflow

## 5. Users and Core Workflows

### Primary users

- Embedded engineers inspecting sensor streams and control loops
- Developers tuning parameters with AI assistance
- Users correlating command input, telemetry changes, and waveform response

### MVP workflows

1. Connect to a device and inspect connection state
2. Start receiving telemetry and display live multi-channel waveforms
3. Select one or more channels and inspect rolling statistics
4. Send commands or parameter changes from the UI
5. Accept or reject AI-generated parameter proposals
6. Capture recent waveform state for AI or human review

## 6. System Blueprint

### High-level flow

```text
Device/Probe
  -> Rust DeviceIO
  -> Framer
  -> Parser
  -> Unified Message Bus / Ring Buffer
  -> UI delivery adapter
  -> Tauri boundary
  -> Frontend buffer/cache
  -> uPlot / Three.js / control panels

                           -> MCP server
                           -> statistics service
                           -> plot snapshot service
```

### Layer responsibilities

#### Rust backend

- Own all transport I/O and parsing
- Normalize telemetry into a stable internal event shape
- Maintain bounded rolling buffers for live consumers
- Provide aggregated statistics for UI and MCP consumers
- Enforce safety gates before hardware-affecting commands execute

#### Tauri boundary

- Carry batched updates from Rust to the frontend
- Carry user commands and approvals from frontend to Rust
- Separate high-rate telemetry transport from low-rate control messages

#### React UI shell

- Own page composition, tool panels, command forms, dialogs, and AI proposal surfaces
- Keep ordinary application state separate from waveform data state
- Avoid driving high-frequency chart updates through deep component rerender chains

#### Visualization layer

- `uPlot`: dense time-series waveform panels
- `Three.js`: orientation / spatial rendering where needed
- `ECharts` optional: lower-frequency summary charts, histograms, and reports

## 7. Core Architecture Decisions

### Decision A: Web UI inside Tauri instead of Rust-native GUI

**Reasoning**

- The product value depends heavily on charting, screenshots, interactive panels, and AI-facing UX
- Web visualization tooling is stronger and faster to productize than current Rust-native alternatives
- Rust still owns the true performance-critical path

**Trade-off**

- Sacrifice: single-language purity
- Gain: chart ecosystem, 3D ecosystem, AI-friendly UI composition, faster iteration

### Decision B: React as orchestration layer, not waveform engine

**Reasoning**

- React is used for application structure and interaction, not for per-sample rendering
- This reduces architectural confusion about where performance work should happen

**Trade-off**

- Sacrifice: some conceptual simplicity versus a pure custom renderer
- Gain: mature component model and easier feature growth

### Decision C: uPlot for primary realtime waveforms

**Reasoning**

- uPlot is optimized for time-series performance and low memory overhead
- It fits the rolling-window telemetry model better than heavier general-purpose charting libraries

**Trade-off**

- Sacrifice: fewer built-in rich dashboard primitives than heavier chart suites
- Gain: lower render cost where it matters most

### Decision D: dual-speed UI architecture

**Reasoning**

- Control panels, dialogs, logs, and AI interactions move at human speed
- Telemetry charts move at machine speed
- These must not share the same update mechanics

**Trade-off**

- Sacrifice: slightly more architectural discipline
- Gain: predictable responsiveness under load

## 8. Data Contracts

### Internal normalized sample

The exact field names may evolve, but the MVP should converge on a stable logical shape:

```text
sample = {
  ts_monotonic,
  channel_id,
  value,
  quality?,
  source?,
  sequence?
}
```

### UI batch payload

Telemetry should cross the Tauri boundary in **batches**, not one sample per UI message.

Logical shape:

```text
batch = {
  start_ts,
  end_ts,
  channels: {
    channel_id -> [values...]
  },
  dropped_count?,
  sequence_range?
}
```

### Statistics contract

Statistics should be provided as a derived service, not recomputed redundantly in the UI.

Logical shape:

```text
stats = {
  channel_id,
  window_ms,
  min,
  max,
  mean,
  variance,
  sample_count
}
```

## 9. Performance Blueprint

### Performance principle

The main risk is **not line drawing alone**. The main risk is repeated cost across the full pipeline:

```text
ingest -> parse -> copy -> serialize -> bridge -> buffer -> rerender
```

The MVP must minimize work at each boundary.

### Required tactics

- Use bounded rolling buffers in Rust
- Push telemetry in batches on a timer or threshold, not every sample
- Keep chart buffers local to the visualization layer
- Use fixed visible time windows by default
- Downsample or decimate for overview displays when channel density grows
- Keep React state for UI metadata, not raw high-rate sample history

### Suggested initial budgets

These are starting budgets for implementation and benchmarking, not hard product promises.

- Sustain `1000Hz+` per active channel in backend ingestion path
- Keep visible waveform refresh around human-smooth rates, for example `20-60 FPS`
- Keep chart view focused on recent seconds rather than unlimited history
- Keep UI command latency perceptibly fast even during streaming

### Measurement-first rule

Before replacing the architecture, benchmark these boundaries separately:

1. Rust ingest and parse throughput
2. Batch serialization and Tauri transport cost
3. Frontend buffer append cost
4. uPlot redraw cost per visible point count
5. Interaction latency while telemetry is active

## 10. UI Composition Blueprint

### Recommended MVP layout

- Left: connection and protocol panel
- Center: primary waveform workspace
- Right: channel inspector, statistics, and AI proposals
- Bottom: command log / device log / parsing diagnostics

### View model split

#### Slow state

- device connection status
- selected port / baud rate
- parser mode
- selected channels
- proposal approval state
- command history metadata

#### Fast state

- rolling waveform samples
- high-rate derived series for current charts
- 3D orientation data updates

These two state classes must remain separate.

## 11. AI Collaboration Blueprint

### AI-facing capabilities for MVP

- Read connection and device metadata
- Request channel statistics for a bounded time window
- Propose parameter changes that require human approval
- Trigger recent waveform snapshot generation

### Approval model

- AI does not directly own hardware mutation by default
- Proposed actions are surfaced in a human-readable review panel
- Operator approval creates an auditable command event

### Snapshot strategy

MVP may support either path:

- Rust-rendered snapshot for deterministic server-side output
- Frontend-captured snapshot for faster UI-aligned implementation

The architecture should isolate this behind a single `plot snapshot` capability so the implementation can change later without breaking the MCP contract.

## 12. Extensibility Hooks

These should be planned now, but only minimally implemented in MVP.

### Hook 1: transport growth

- Serial first
- Future: UDP/TCP/CAN/probe-rs

### Hook 2: parser growth

- Text key-value first
- Future: JSON, binary frames, structured C payloads

### Hook 3: visualization growth

- Primary realtime waveform now
- Future: overlays, annotations, events, replay, comparative sessions

### Hook 4: alternate frontend research

- Preserve a boundary where `egui` or other Rust-native visualization experiments can consume the same normalized telemetry stream for benchmarking

## 13. Risks and Mitigations

### Risk: frontend rerender storms

- Mitigation: isolate waveform data outside general component state

### Risk: Tauri bridge becomes bottleneck

- Mitigation: batch aggressively, measure transport overhead early, keep payload contracts compact

### Risk: chart performance collapses with too many visible points

- Mitigation: fixed windows, channel selection, overview decimation, adaptive detail levels

### Risk: AI actions feel unsafe

- Mitigation: explicit approval UI, limits, audit trail, emergency stop path

### Risk: MVP overbuilds configurability

- Mitigation: ship opinionated defaults first, expose extension points without implementing all of them

## 14. Alternatives and When to Revisit

### Revisit `egui` if

- the product shifts toward an internal engineering tool with lower emphasis on web-style extensibility
- benchmark data shows Tauri boundary cost is unacceptable even after batching
- the team strongly prefers all-Rust development over web ecosystem leverage

### Revisit `iced` if

- the project prioritizes a more structured Rust-native app model over chart ecosystem speed
- custom rendering and application architecture converge around a pure Rust desktop stack

### Revisit a different chart stack if

- uPlot fails practical interaction needs
- 3D and chart composition require tighter rendering integration than separate libraries can provide

## 15. MVP Milestones

### Milestone 1: shell and data path

- Tauri shell boots
- Rust backend emits normalized telemetry batches
- Frontend receives and displays a single live waveform

### Milestone 2: operator workflow

- multi-channel selection
- rolling statistics panel
- command send and response log
- AI proposal approval dialog

### Milestone 3: AI-visible visualization

- snapshot generation path
- MCP endpoint integration for statistics and snapshots
- performance instrumentation on main boundaries

## 16. Open Questions Left Deliberately Open

These should not block the blueprint, but are good research directions:

- exact Tauri transport primitive: event, channel, or custom strategy
- whether overview charts should use decimated data or a separate summary pipeline
- whether snapshots should be generated in Rust first or in the frontend first
- whether React state should remain minimal plus local refs, or adopt a lightweight store for UI shell state

## 17. Final Recommendation

Build the MVP as a **Rust-first telemetry engine with a Web-first visualization surface**.

The important architectural rule is:

> Keep high-rate telemetry on a dedicated path, and keep ordinary UI concerns off that path.

If that rule is preserved, the chosen stack remains flexible enough for future optimization, chart swaps, or Rust-native experiments without invalidating the MVP.
