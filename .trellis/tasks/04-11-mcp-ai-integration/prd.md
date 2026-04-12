# MCP and AI Interaction Integration

## Goal
Expose the existing telemetry runtime to MCP and AI clients through a small read-only integration layer.

## Requirements
- Provide a stable AI-facing contract for session state, recent telemetry, analysis frames, and trigger history.
- Keep MCP access read-only for the first version.
- Reuse existing backend message/analysis structures instead of introducing a second data model.
- Avoid coupling MCP directly to serial transport internals or UI state.
- Preserve current runtime, fake stream, and desktop UI behavior.

## Acceptance Criteria
- [ ] A clear MCP adapter boundary exists in the codebase.
- [ ] AI can query current session and recent telemetry summaries without parsing raw serial text.
- [ ] AI can observe analysis and trigger events in a structured form.
- [ ] Existing runtime and UI behavior remain unchanged.
- [ ] Tests cover the new AI-facing contract or adapter behavior.

## Technical Notes
Start with read-only MCP resources/tools. Prefer a thin adapter over a large framework integration. The main design decision is what data shape AI should consume: raw events, summarized telemetry, or both.
