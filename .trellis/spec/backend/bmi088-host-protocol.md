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

### Event contract

- `IDENTITY=0x82` is parsed as a TLV event frame.
- `SCHEMA=0x80` is parsed as a structured schema frame.
- `SAMPLE=0x81` is parsed using the latest schema when available, otherwise the built-in default schema.
- `SHELL_OUTPUT=0x83` is parsed as raw line payload bytes plus lossy UTF-8 text.
- Unknown event commands stay decodable as unknown frames at the protocol layer, but do not become typed BMI088 events.

### Identity TLV contract

- Required TLVs: identity format version, device name, board name, firmware version, protocol name, protocol version, transport name, sample rate, schema field count, sample payload len, protocol version byte, feature flags, baud rate, protocol minor version.
- Missing required TLVs, truncated TLV headers, or truncated TLV values are malformed frames.
- The current identity contract must report schema field count `19` and sample payload length `38`.

### Schema/sample contract

- Sample schema is 19 ordered `i16` fields.
- Sample payload length is 38 bytes.
- Field order is fixed by the active schema and must match decoder order.
- `SCHEMA` frames must not contain trailing bytes.
- Sample payload length must be exactly `field_count * 2`.

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
- Sample payload with odd byte count -> malformed sample frame.
- UI payload provided for non-payload commands -> ignored, not an error.
- Runtime command issued when session is not connected/running -> not connected error.

---

## 5. Good/Base/Bad Cases

- Good: UI sends `REQ_IDENTITY`, host emits an `IDENTITY` TLV event, then `REQ_SCHEMA`, then `ACK` and `START`.
- Good: UI sends `SHELL_EXEC` with payload bytes; host emits `SHELL_OUTPUT` as a typed event.
- Base: `REQ_TUNING` and `SET_TUNING` share the same request frame path as the other BMI088 host commands.
- Bad: keep assuming a 9-field sample layout after the identity/schema handshake now advertises 19 fields and 38 bytes.
- Bad: treat `SHELL_OUTPUT` as plain text-only logging and drop the raw payload.

---

## 6. Tests Required

- Protocol encode/decode tests must assert the request codes for `REQ_IDENTITY`, `REQ_TUNING`, `SET_TUNING`, and `SHELL_EXEC`.
- Identity tests must assert TLV decoding, required fields, and the `schema_field_count=19` / `sample_payload_len=38` contract.
- Schema/sample tests must assert 19 fields, 38-byte sample length, schema-order decoding, and CRC rejection.
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
// Correct: only payload-bearing commands forward bytes, and the active BMI088 schema is 19 fields.
host.send_bmi088_command(Bmi088HostCommand::ReqIdentity, None)?;
host.send_bmi088_command(Bmi088HostCommand::ShellExec, Some(b"help".to_vec()))?;
assert_eq!(sample.fields.len(), 19);
assert_eq!(identity.schema_field_count, 19);
assert_eq!(identity.sample_payload_len, 38);
```
