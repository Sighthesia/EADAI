# Technical Design

## Implementation Strategy
- Add a dedicated BMI088 binary protocol module in Rust for:
  - CRC16 helper
  - frame encode/decode
  - schema descriptor parsing
  - sample decoding and scaling metadata
- Add a dual-mode serial ingest path:
  - text line framing for existing protocols
  - binary frame scanning for BMI088
  - ASCII command buffering only while parser is idle and outside binary frames
- Add a runtime session state machine for BMI088 handshake and re-handshake.
- Extend the message bus with structured schema/sample events.
- Keep backward compatibility by also projecting decoded sample fields into the existing variable/analysis pipeline as normalized channel/value/unit parser fields.
- Extend Tauri/UI types and store to ingest structured schema/sample events.
- Add a lightweight app scripting callback registry with `onSchema` and `onSample` hooks.
- Ensure the shared AI adapter and MCP boundary consume decoded telemetry from the same backend bus.

## Suggested Areas To Touch
- `src/serial.rs`
- `src/app.rs`
- `src/message.rs`
- `src/runtime_host.rs`
- `src/fake_session.rs`
- `src-tauri/src/model.rs`
- `src-tauri/src/state.rs`
- `ui/src/types.ts`
- `ui/src/store/appStore.ts`
- New Rust modules for BMI088 protocol/session handling
- New tests under `tests/`

## Good / Base / Bad
- Good: open port, receive schema, send ACK + START, stream 100 Hz samples.
- Base: reconnect and force `REQ_SCHEMA`, then re-ack and restart.
- Bad: assume fixed sample layout without schema, send `START` before `ACK`, treat angles as floats, or mix ASCII bytes into a binary frame.
