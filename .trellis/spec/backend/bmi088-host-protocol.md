# BMI088 Host Protocol Contract

> Executable contract for the BMI088 UART4 host runtime, from Rust protocol parsing to Tauri/UI command dispatch.

---

## 1. Scope / Trigger

- Trigger: the host protocol gained `REQ_IDENTITY`, identity TLV parsing, `REQ_TUNING`, `SET_TUNING`, `SHELL_EXEC`, `SHELL_OUTPUT`, and payload-bearing command support across Rust, Tauri, and UI.
- Scope: one shared BMI088 binary/text ingest path, one host command encoding path, and one UI request path.
- This spec is the source of truth for the executable wire contract; stale task PRDs may still mention the older 9-field / 100000 baud bringup flow.

---

## 2. Signatures

- Rust commands: `Bmi088HostCommand::{Ack, Start, Stop, ReqSchema, ReqIdentity, ReqTuning, SetTuning, ShellExec}`.
- Rust encoder: `encode_host_command(command)`, `encode_host_command_with_payload(command, payload)`, `encode_host_command_with_seq_and_payload(command, seq, payload)`.
- Rust decoder outputs: `Bmi088Frame::{Identity, Schema, Sample, ShellOutput}` and `TelemetryPacket::{Text, ShellOutput, Identity, Schema, Sample}`.
- Runtime entry points: `SessionRuntimeHost::send_bmi088_command(...)` and `DesktopState::send_bmi088_command(...)`.
- UI request shape: `Bmi088CommandRequest { command, payload? }` and `UiBmi088HostCommand`.

---

## 3. Contracts

### Frame envelope

- SOF: `0xA5 0x5A`.
- Version: `0x01`.
- Frame types: `REQUEST=0x01`, `RESPONSE=0x02`, `EVENT=0x03`.
- CRC: CRC16-CCITT, init `0xFFFF`, poly `0x1021`, little-endian on wire.
- Header size: 7 bytes. CRC size: 2 bytes.

### Host command contract

- `ACK=0x10`, `START=0x11`, `STOP=0x12`, `REQ_SCHEMA=0x13`, `REQ_IDENTITY=0x14`.
- `REQ_TUNING=0x26`, `SET_TUNING=0x27`, `SHELL_EXEC=0x28`.
- Host commands are encoded as `REQUEST` frames.
- Payload-bearing commands on this path are only `SET_TUNING` and `SHELL_EXEC`.
- `UiBmi088HostCommand::payload_bytes()` must return bytes only for `SET_TUNING` and `SHELL_EXEC`; other commands must ignore any UI payload.
- Session startup boot traffic must auto-send only `REQ_SCHEMA`.
- `REQ_IDENTITY` remains a supported manual discovery request, but it is not part of the mandatory startup handshake.

### Event contract

- `IDENTITY=0x82`, `SCHEMA=0x80`, and `SAMPLE=0x81` should be accepted from `EVENT(0x03)` frames.
- For backward compatibility with older firmware/docs, the host should also accept these typed telemetry frames from `RESPONSE(0x02)`.
- `IDENTITY=0x82` is parsed as a TLV telemetry frame.
- `SCHEMA=0x80` is parsed as a structured schema frame.
- `SAMPLE=0x81` is parsed using the latest schema when available, otherwise the built-in default schema.
- `SHELL_OUTPUT=0x83` is parsed as raw line payload bytes plus lossy UTF-8 text.
- Unknown event commands stay decodable as unknown frames at the protocol layer, but do not become typed BMI088 events.

### Identity TLV contract

- Required TLVs: identity format version, device name, board name, firmware version, protocol name, protocol version, transport name, sample rate, schema field count, sample payload len, protocol version byte, feature flags, baud rate, protocol minor version.
- Missing required TLVs, truncated TLV headers, or truncated TLV values are malformed frames.
- The current identity contract must report schema field count `30` and sample payload length `60`.

### Schema/sample contract

- The host must support more than one schema payload layout.
- Modern schema layout: `schema_version | rate_hz | field_count | sample_len | descriptors...`.
- Legacy device layout: schema payload may omit the 4-byte top-level header and instead carry descriptor records directly.
- Observed mixed legacy layout: the first field may use a short descriptor (`scale_q | name_len | unit_len | name | unit`), while later fields may use a 5-byte descriptor header (`field_id | field_type | scale_q | name_len | unit_len | name | unit`).
- Sample payload length remains `field_count * 2` for all supported layouts.
- Field order is fixed by the active schema and must match decoder order.
- `SCHEMA` frames must not contain trailing bytes once decoded under the chosen schema variant.
- Before streaming starts, a valid `SCHEMA` advances the host session to `AwaitingAck` and emits `ACK` then `START`.
- After the session is already `Streaming` or `Stopped`, duplicate `SCHEMA` frames may refresh cached schema metadata but must not retrigger `ACK`/`START` automatically.
- Pre-stream recovery may retry `REQ_SCHEMA` while the host is still in `AwaitingSchema`; startup boot emission and retry emission are separate paths.

### Cross-layer projection

- Rust bus emits `TelemetryIdentity`, `TelemetrySchema`, `TelemetrySample`, and `ShellOutput` so the UI can consume typed events without re-decoding the wire format.
- `SessionRuntimeHost` forwards host commands over the active session; the desktop layer does not invent a second BMI088 command encoder.

---

## 4. Validation & Error Matrix

- Bad SOF -> `InvalidSof` and stream resync.
- Bad version -> `InvalidVersion`.
- Bad CRC -> `InvalidCrc` and stream resync.
- Unsupported BMI088 command -> protocol-level unknown frame or malformed event, never a typed identity/schema/sample/shell output event.
- Identity TLV truncated or missing required field -> malformed identity frame.
- Schema trailing bytes -> malformed schema frame.
- Schema field count != sample payload length / 2 -> schema mismatch.
- Legacy schema descriptor underflow or invalid UTF-8 -> malformed schema frame.
- Sample payload with odd byte count -> malformed sample frame.
- Repeated `SCHEMA` while already streaming -> refresh cached schema only, no new outbound `ACK`/`START` pair.
- Startup boot path called twice in one connected session -> no second automatic `REQ_SCHEMA` emission.
- UI payload provided for non-payload commands -> ignored, not an error.
- Runtime command issued when session is not connected/running -> not connected error.

---

## 5. Good/Base/Bad Cases

- Good: startup sends `REQ_SCHEMA`, host receives `SCHEMA`, then emits `ACK` and `START` once.
- Good: UI may send `REQ_IDENTITY` after connect to discover device metadata without changing the startup handshake path.
- Good: UI sends `SHELL_EXEC` with payload bytes; host emits `SHELL_OUTPUT` as a typed event.
- Base: `REQ_TUNING` and `SET_TUNING` share the same request frame path as the other BMI088 host commands.
- Bad: assume every device emits the same top-level `SCHEMA` header layout.
- Bad: keep assuming a 9-field sample layout after the identity/schema handshake now advertises 30 fields and 60 bytes.
- Bad: treat `SHELL_OUTPUT` as plain text-only logging and drop the raw payload.
- Bad: send `REQ_IDENTITY` as part of the mandatory boot handshake and disturb the MCU's `SCHEMA -> ACK -> START` startup path.
- Bad: reply to every later `SCHEMA` with a fresh `ACK`/`START` even after the session is already streaming.

---

## 6. Tests Required

- Protocol encode/decode tests must assert the request codes for `REQ_IDENTITY`, `REQ_TUNING`, `SET_TUNING`, and `SHELL_EXEC`.
- Identity tests must assert TLV decoding, required fields, and the `schema_field_count=30` / `sample_payload_len=60` contract.
- Schema/sample tests must assert 30 fields, 60-byte sample length, schema-order decoding, and CRC rejection.
- Schema/sample tests must also cover at least one legacy schema payload variant observed from the current device.
- Session tests must assert startup boot emits only one automatic `REQ_SCHEMA`, schema retry remains available only while `AwaitingSchema`, and duplicate `SCHEMA` frames do not restart `ACK`/`START` after streaming.
- Runtime/UI tests must assert `UiBmi088HostCommand` only forwards payload bytes for `SET_TUNING` and `SHELL_EXEC`.
- Bus projection tests must assert `TelemetryIdentity`, `TelemetrySchema`, `TelemetrySample`, and `ShellOutput` are preserved as distinct event kinds.

---

## 7. Wrong vs Correct

### Wrong

```rust
// Wrong: assume all commands can carry a payload and that samples are still 9 fields.
host.send_bmi088_command(Bmi088HostCommand::ReqIdentity, Some(payload))?;
assert_eq!(sample.fields.len(), 9);
```

### Correct

```rust
// Correct: only payload-bearing commands forward bytes, and the active BMI088 schema is 30 fields.
host.send_bmi088_command(Bmi088HostCommand::ReqSchema, None)?;
host.send_bmi088_command(Bmi088HostCommand::ShellExec, Some(b"help".to_vec()))?;
assert_eq!(sample.fields.len(), 30);
assert_eq!(identity.schema_field_count, 30);
assert_eq!(identity.sample_payload_len, 60);
```
