# Desktop AI Panel with Shared Runtime Context

## Goal
Add an in-app AI panel to the desktop shell that reads the same live runtime context used by MCP, so the UI and AI see one shared telemetry view.

## Requirements
- Reuse the existing AI adapter and telemetry summary contract from the backend.
- Keep the panel read-only for the first version.
- Show current session state, recent telemetry summaries, latest analysis frames, and trigger history.
- Avoid duplicating runtime parsing or maintaining a second state store.
- Preserve the current desktop shell and serial behavior.

## Acceptance Criteria
- [ ] The desktop shell can read the shared AI context.
- [ ] The panel renders live session and telemetry snapshots.
- [ ] Analysis and trigger history are visible in structured form.
- [ ] Existing runtime behavior remains unchanged.
- [ ] Tests cover the shared context path or UI ingestion path.

## Technical Notes
Prefer wiring the panel to the shared backend adapter rather than the MCP process. The main design choice is how much of the AI-facing contract the panel should render directly versus summarize again for the UI.
