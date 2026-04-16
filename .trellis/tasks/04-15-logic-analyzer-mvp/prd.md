# Logic analyzer MVP with sigrok sidecar

## Goal
Build an MVP logic analyzer workspace that integrates with sigrok through an external sidecar process in the existing Rust + Tauri + React app. The first release should focus on capture workflow and operational visibility on Linux, without protocol decoding.

## Requirements
- Use sigrok as the capture backend via a sidecar/external process, not by embedding libsigrok.
- Prioritize Linux support for the MVP.
- Provide a dedicated logic analyzer page/workspace separate from the existing oscilloscope workflow.
- Support sigrok availability probing.
- Support device scanning.
- Support starting and stopping captures.
- Show capture status in the UI.
- Surface backend and sidecar errors clearly in the UI.
- Keep the scope to acquisition only; protocol decoding is out of scope for this phase.
- Allow a later incremental step to show a minimal digital waveform after capture completes, but do not require full real-time streaming in this MVP.

## Acceptance Criteria
- [ ] The app can detect whether sigrok is available on the host.
- [ ] The app can list available logic analyzer devices discovered through sigrok.
- [ ] The user can start and stop a capture from the dedicated logic analyzer workspace.
- [ ] The UI reflects current capture state and any failures.
- [ ] Sidecar/process errors are returned to the frontend in a readable form.
- [ ] The implementation fits the existing Rust + Tauri + React architecture.
- [ ] No protocol decoding UI or logic is included.

## Technical Notes
- Prefer a narrow command/API contract between the Tauri layer and the sigrok sidecar so the capture backend remains replaceable.
- Keep the initial data model focused on probe result, device list, capture state, and error payloads.
- Linux-first means the first implementation may assume Linux-friendly sigrok tooling and process invocation paths.
- If waveform rendering is added later, it should be limited to a post-capture minimal digital view rather than a full analyzer pipeline.
