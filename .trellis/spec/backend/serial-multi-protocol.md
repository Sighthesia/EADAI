# Serial Multi-Protocol Contract

> Executable contract for built-in serial protocol ingest beyond plain text, including BMI088, MAVLink, and Crazyflie CRTP-over-serial.

---

## 1. Scope / Trigger

- Trigger: the serial runtime now supports multiple built-in protocols on the same backend ingest path.
- Scope: parser selection, auto-detect behavior, protocol decoder boundaries, Rust bus events, and Tauri/UI projections.
- This spec exists to prevent future sessions from conflating `CRTP-over-serial` support with full Crazyflie transport support.

---

## 2. Signatures

- CLI parser modes: `ParserKind::{Auto, KeyValue, Measurements, Bmi088, Mavlink, Crtp}`.
- Runtime decoder entry points:
  - `Bmi088StreamDecoder::push(chunk)`
  - `MavlinkDecoder::push(chunk)`
  - `CrtpDecoder::push(chunk)`
- Rust bus event kinds:
  - `MessageKind::TelemetryIdentity`
  - `MessageKind::TelemetrySchema`
  - `MessageKind::TelemetrySample`
  - `MessageKind::MavlinkPacket`
  - `MessageKind::CrtpPacket`
- Tauri/UI event kinds:
  - `UiBusEvent::TelemetryIdentity`
  - `UiBusEvent::TelemetrySchema`
  - `UiBusEvent::TelemetrySample`
  - `UiBusEvent::MavlinkPacket`
  - `UiBusEvent::CrtpPacket`

---

## 3. Contracts

### Built-in protocol scope

- `Mavlink` means serial MAVLink frame detection and parsing in the Rust backend.
- `Crtp` means Crazyflie `CRTP-over-serial` only.
- `Crtp` does **not** imply Crazyradio dongle support.
- `Crtp` does **not** imply Crazyflie USB native transport support.
- CLI or docs must not claim full Crazyflie support unless transport work is implemented separately.

### Auto-detect behavior

- `ParserKind::Auto` must try binary decoders before falling back to text parsing.
- Current required detect order is:
  1. BMI088
  2. MAVLink
  3. CRTP-over-serial
  4. Text auto-parse
- Auto mode must tolerate garbage and partial chunks without panicking.
- Auto mode must keep false positives low enough that ordinary text does not frequently surface as MAVLink or CRTP packets.

### Cross-layer projection

- Binary protocol decode happens in Rust, not in the UI.
- Tauri/UI receives normalized packet events, not raw protocol byte streams requiring re-decode.
- MAVLink UI packet shape must include:
  - `sequence`
  - `system_id`
  - `component_id`
  - `message_id`
  - `payload_len`
  - `fields`
- CRTP UI packet shape must include:
  - `port`
  - `channel`
  - `payload_len`
  - `fields`

### Semantic field projection

- Common MAVLink messages should project stable semantic field keys through `fields` instead of leaving the UI to infer payload layout.
- Current required MAVLink semantic coverage:
  - `HEARTBEAT (0x0000)` -> `type`, `autopilot`, `base_mode`, `custom_mode`, `system_status`
  - `SYS_STATUS (0x0001)` -> `voltage_battery`, `current_battery`, `battery_remaining`, `drop_rate_comm`
  - `SYSTEM_TIME (0x0002)` -> `time_unix_usec`, `time_boot_ms`
  - `GPS_RAW_INT (0x0021)` -> `fix_type`, `lat`, `lon`, `alt`, `satellites_visible`
  - `ATTITUDE_QUATERNION (0x0033)` -> `q1`, `q2`, `q3`, `q4`, `rollspeed`, `pitchspeed`, `yawspeed`
  - `LOCAL_POSITION_NED (0x0035)` -> `x`, `y`, `z`, `vx`, `vy`, `vz`, `time_boot_ms`
  - `GLOBAL_POSITION_INT (0x0039)` -> `lat`, `lon`, `alt`, `relative_alt`, `vx`, `vy`, `vz`, `hdg`
  - `COMMAND_ACK (0x0053)` -> `command`, `result`
  - `VFR_HUD (0x0051)` -> `airspeed`, `groundspeed`, `heading`, `throttle`, `alt`, `climb`
  - `RC_CHANNELS (0x00A0)` -> `ch1`-`ch18`, `rssi`, `time_boot_ms`
  - `HIGHRES_IMU (0x00BE)` -> `xacc`, `yacc`, `zacc`, `xgyro`, `ygyro`, `zgyro`, `xmag`, `ymag`, `zmag`
  - `GPS_STATUS (0x00C7)` -> `satellites_visible`, `satellite_count`
  - `SCALED_PRESSURE (0x00C9)` -> `press_abs`, `press_diff`, `temperature`, `time_boot_ms`
  - `ATTITUDE (0x00CA)` -> `roll`, `pitch`, `yaw`, `rollspeed`, `pitchspeed`, `yawspeed`
  - `BATTERY_STATUS (0x00D0)` -> `battery_function`, `battery_type`, `temperature`, `voltage`, `current`, `remaining`
  - `AUTOPILOT_VERSION (0x00D1)` -> `flight_sw_version`, `middleware_sw_version`, `os_sw_version`
  - `VIBRATION (0x00FE)` -> `vibration_x`, `vibration_y`, `vibration_z`, `clipping_0`, `clipping_1`, `clipping_2`
- Unknown MAVLink message IDs may stay generic, but they must still preserve the envelope metadata fields.

- Common CRTP ports/channels should also project stable semantic field keys through `fields`.
- Current required CRTP semantic coverage:
  - `console` -> `text`
  - `parameter` -> `operation`, `param_id`, `param_value`, `toc_cmd`
  - `commander` -> `control_mode`, plus mode-specific keys such as `roll`, `pitch`, `yaw`, `thrust`, `height`, `vx`, `vy`, `yaw_rate`, `command`
  - `memory` -> `operation`, `memory_cmd`, `status`
  - `logging` -> `log_type`, `command`, `log_channel`, `log_id`
  - `high_level_commander` -> `command_type`, `command`, and trajectory-style coordinates such as `x`, `y`, `z`
  - `setting` -> `operation`, `setting_id`, `value`
  - `debug` -> `text`
- Unknown CRTP ports or channels may stay generic, but they must still preserve `port`, `channel`, and `payload_len`.

### Basic display contract

- The existing variable/console display path must consume semantic fields directly for common MAVLink and CRTP packets.
- UI formatting helpers may derive human-readable strings from `fields`, but they must not reverse-engineer binary payloads on the frontend.
- If no semantic formatter exists for a packet type, the UI may fall back to a concise generic summary instead of failing the event.

### Text coexistence contract

- Binary protocol support must not remove the existing text line path.
- When no binary decoder claims a chunk in auto mode, the runtime must still publish text-compatible line events.
- BMI088 embedded text lines still flow through the text parse/analysis path after BMI088 framing.

---

## 4. Validation & Error Matrix

- Invalid or truncated MAVLink frame -> drop/resync at decoder level, never panic.
- Invalid or truncated CRTP-over-serial frame -> drop/resync at decoder level, never panic.
- Unknown MAVLink message ID without known CRC extra -> packet may be surfaced as untyped/weakly validated, but must not masquerade as a different protocol.
- Garbage bytes before a valid binary frame -> skipped until sync is re-established.
- Auto mode receives ordinary text -> must fall through to text parsing instead of inventing protocol packets.
- User selects `--parser crtp` -> behavior is limited to CRTP-over-serial; no transport side effects or Crazyradio assumptions.
- Common-message semantic projection missing required keys -> contract drift; fix the Rust decoder mapping instead of patching around it in UI-only code.

---

## 5. Good/Base/Bad Cases

- Good: `--parser auto` on a MAVLink serial stream emits `MavlinkPacket` events into Rust, Tauri, and UI.
- Good: `--parser auto` on a CRTP-over-serial stream emits `CrtpPacket` events into Rust, Tauri, and UI.
- Good: `--parser auto` on legacy text telemetry still emits text line events and analysis.
- Base: `--parser mavlink` and `--parser crtp` are explicit receive-side protocol modes, not promises about outbound control features.
- Bad: describe `crtp` support as "Crazyflie supported" without stating it is serial-only.
- Bad: move MAVLink or CRTP decode logic into the frontend.
- Bad: let auto mode greedily claim arbitrary text as binary packets without CRC or framing evidence.

---

## 6. Tests Required

- Decoder tests must cover good frames, bad CRC/checksum, garbage before sync, and partial chunk assembly for both MAVLink and CRTP.
- Runtime tests should preserve existing text parser behavior in auto mode.
- Cross-layer tests should assert new bus event kinds serialize through Tauri models without shape drift.
- Semantic mapping tests should assert the required `fields` keys for the covered MAVLink message IDs and CRTP ports/channels.
- CLI tests should assert `--parser mavlink` and `--parser crtp` parse successfully.
- Any future transport work for Crazyradio or USB must add new tests and must not silently piggyback on this serial-only contract.

---

## 7. Wrong vs Correct

### Wrong

```rust
// Wrong: equate CRTP parser mode with complete Crazyflie support.
match parser {
    ParserKind::Crtp => connect_crazyradio_and_decode()?,
    _ => run_serial_runtime()?,
}
```

### Correct

```rust
// Correct: CRTP mode is a serial decoder choice only.
match parser {
    ParserKind::Crtp => decode_crtp_over_serial(chunk),
    ParserKind::Mavlink => decode_mavlink(chunk),
    _ => run_existing_paths(chunk),
}
```
