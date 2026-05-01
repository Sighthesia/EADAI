# Serial Multi-Protocol Contract

> Executable contract for built-in protocol ingest beyond plain text, including BMI088, MAVLink, Crazyflie CRTP, and Crazyradio transport.

---

## 1. Scope / Trigger

- Trigger: the runtime now supports multiple built-in protocols on the same backend ingest path, with explicit transport/framing/semantic/capability layering.
- Scope: transport selection, parser selection, auto-detect behavior, protocol decoder boundaries, capability event mapping, Rust bus events, and Tauri/UI projections.
- This spec documents the layered architecture for protocol support and the boundary between serial-only and Crazyradio transport.

---

## 2. Architecture Layers

The protocol support is organized into four explicit layers:

```
+-------------------+
|    Capability     |  <- Cross-protocol business events (Attitude, Battery, GPS, etc.)
+-------------------+
|     Semantic      |  <- Protocol-specific field extraction (MAVLink fields, CRTP ports)
+-------------------+
|     Framing       |  <- Frame detection and extraction (MAVLink v2, CRTP-over-serial)
+-------------------+
|    Transport      |  <- Byte stream (Serial, Crazyradio, Fake)
+-------------------+
```

### Transport Layer (`src/protocols/transport.rs`)

- `ByteTransport` trait: provides `read_chunk()`, `write_all()`, `flush()`, `is_connected()`
- `TransportKind` enum: `Serial`, `Crazyradio`, `CrazyflieUsb`, `Fake`
- Implementations:
  - `SerialTransport` (`src/protocols/serial_transport.rs`): wraps `serialport::SerialPort`
  - `CrazyradioTransport` (`src/protocols/crazyradio.rs`): wraps `crazyflie-link` async link
- The transport layer is protocol-agnostic; it only provides byte streams.

### Framing Layer (`src/protocols/mavlink.rs`, `src/protocols/crtp.rs`)

- `MavlinkDecoder::push(chunk) -> Vec<MavlinkPacket>`: MAVLink v2 frame detection with CRC validation
- `CrtpDecoder::push(chunk) -> Vec<CrtpPacket>`: CRTP-over-serial frame detection with CRC-8 validation
- Decoders are stateful and handle partial frame assembly across chunks.

### Semantic Layer (within decoder `fields()` methods)

- `MavlinkPacket::fields() -> BTreeMap<String, String>`: extracts semantic fields for known message IDs
- `CrtpPacket::fields() -> BTreeMap<String, String>`: extracts semantic fields for known CRTP ports
- Semantic mapping is集中 in the Rust backend, not in the UI.

### Capability Layer (`src/protocols/capability.rs`)

- `CapabilityEvent` enum: cross-protocol business events
  - `Attitude(AttitudeData)`: roll, pitch, yaw
  - `BatteryStatus(BatteryData)`: voltage, current, remaining
  - `GpsPosition(GpsData)`: lat, lon, alt, satellites
  - `ImuData(ImuData)`: accel, gyro, mag
  - `LocalPosition(LocalPositionData)`: x, y, z, vx, vy, vz
  - `SystemStatus(SystemStatusData)`: system health
  - `RawPacket(RawPacketData)`: debug/protocol-specific display
- `mavlink_to_capabilities(packet)`: maps MAVLink messages to capability events
- `crtp_to_capabilities(packet)`: maps CRTP packets to capability events
- Capability events are published to the bus alongside raw protocol events.

---

## 3. Signatures

- CLI parser modes: `ParserKind::{Auto, KeyValue, Measurements, Bmi088, Mavlink, Crtp}`.
- CLI transport selection: `TransportSelection::{Serial, Crazyradio { uri }}`.
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
  - `MessageKind::ProtocolDetected { protocol }`
  - `MessageKind::Capability` (new)
- Tauri/UI event kinds:
  - `UiBusEvent::TelemetryIdentity`
  - `UiBusEvent::TelemetrySchema`
  - `UiBusEvent::TelemetrySample`
  - `UiBusEvent::MavlinkPacket`
  - `UiBusEvent::CrtpPacket`
  - `UiBusEvent::ProtocolDetected { protocol }`
  - `UiBusEvent::Capability` (new)

---

## 4. Contracts

### Transport scope

- `TransportSelection::Serial` is the default transport; uses `serialport` crate.
- `TransportSelection::Crazyradio { uri }` uses `crazyflie-link` crate for real Crazyradio dongle communication.
- `CrazyradioTransport` is a real, runnable transport backed by the `crazyflie-link` async link.
- `CrazyflieUsb` is reserved for future USB native transport; not yet implemented.
- CLI `--transport serial` and `--transport crazyradio --radio-uri <uri>` select the transport.
- Rust bus `MessageSource.transport` must project `TransportKind::{Serial, Crazyradio, Fake}` consistently into Tauri `UiTransportKind`.

### Built-in protocol scope

- `Mavlink` means MAVLink v2 frame detection and parsing in the Rust backend.
- `Crtp` means Crazyflie `CRTP-over-serial` framing.
- `Crtp` framing works over both serial and Crazyradio transports.
- CLI or docs must not claim "complete Crazyflie support" unless all transports and capabilities are implemented.

### Auto-detect behavior

- `ParserKind::Auto` must try binary decoders before falling back to text parsing.
- Current required detect order is:
  1. BMI088
  2. MAVLink
  3. CRTP-over-serial
  4. Text auto-parse
- Auto mode must tolerate garbage and partial chunks without panicking.
- Auto mode must keep false positives low enough that ordinary text does not frequently surface as MAVLink or CRTP packets.
- Auto mode must not let an earlier successful decoder suppress later binary decoders for the same chunk; BMI088, MAVLink, and CRTP decoders each get a chance to inspect the bytes.
- `ProtocolDetected` is emitted once per chunk-processing pass on the first protocol that successfully produced structured packets; it is a UI/debug hint, not an exclusive session lock.
- CRTP packets detected in auto mode must still pass through `is_self_describing_packet()` so the self-describing session path remains active without requiring explicit `--parser crtp`.

### Desktop/Tauri connect contract

- `src-tauri/src/model/session.rs::ConnectRequest` accepts:
  - `parser?: "auto" | "bmi088" | "mavlink" | "crtp" | "key_value" | "measurements"`
  - `transport?: "serial" | "crazyradio"`
  - `radioUri?: string`
- Missing `parser` keeps the backward-compatible default `bmi088`.
- Missing `transport` defaults to `serial`.
- When `transport == "crazyradio"`, `radioUri` is the intended connection target and must be forwarded into `TransportSelection::Crazyradio { uri }` instead of being ignored by the desktop state layer.

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
- Protocol-detected UI events must include:
  - `protocol`
- Capability events include `source_protocol` field to identify the originating protocol.

### Capability event contract

- Capability events are published for every MAVLink/CRTP packet that maps to a known capability.
- Raw protocol events are always published (for debug and protocol-specific display).
- UI may consume capability events for cross-protocol unified display.
- Capability events include `source_protocol` ("mavlink" or "crtp") for traceability.
- CRTP commander RPYT packets must map to `CapabilityEvent::Attitude` in addition to `RawPacket`, so the unified attitude path is available even without MAVLink.

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

## 5. Validation & Error Matrix

- Invalid or truncated MAVLink frame -> drop/resync at decoder level, never panic.
- Invalid or truncated CRTP-over-serial frame -> drop/resync at decoder level, never panic.
- Unknown MAVLink message ID without known CRC extra -> packet may be surfaced as untyped/weakly validated, but must not masquerade as a different protocol.
- Garbage bytes before a valid binary frame -> skipped until sync is re-established.
- Auto mode receives ordinary text -> must fall through to text parsing instead of inventing protocol packets.
- Auto mode receives a chunk where one decoder succeeds -> later decoders may still inspect the same bytes; do not short-circuit on `handled = true` style logic.
- User selects `--parser crtp` -> behavior is limited to CRTP framing; transport is selected separately via `--transport`.
- Desktop connect request sets `transport = "crazyradio"` but omits `radioUri` -> runtime currently forwards an empty URI; UI should treat this as invalid input even if backend fallback behavior still exists.
- Crazyradio connection failure -> transport returns `TransportError::ConnectionFailed`; runtime retries via reconnect controller.
- Common-message semantic projection missing required keys -> contract drift; fix the Rust decoder mapping instead of patching around it in UI-only code.

---

## 6. Good/Base/Bad Cases

- Good: `--parser auto --transport serial` on a MAVLink serial stream emits `MavlinkPacket` + `Capability` events.
- Good: `--parser auto --transport serial` on a CRTP-over-serial stream emits `CrtpPacket` + `Capability` events.
- Good: `--parser auto` on a self-describing CRTP stream emits `ProtocolDetected`, raw `CrtpPacket`, and self-describing events without requiring a parser switch.
- Good: `--transport crazyradio --radio-uri radio://0/60/2M/E7E7E7E7E7` connects via Crazyradio dongle.
- Good: `--parser auto` on legacy text telemetry still emits text line events and analysis.
- Base: `--parser mavlink` and `--parser crtp` are explicit framing modes, not promises about outbound control features.
- Base: `ProtocolDetected` is informative for UI/debug state; packet and capability events remain the authoritative data path.
- Bad: describe `crtp` support as "Crazyflie supported" without stating transport scope.
- Bad: move MAVLink or CRTP decode logic into the frontend.
- Bad: let auto mode greedily claim arbitrary text as binary packets without CRC or framing evidence.

---

## 7. Tests Required

- Decoder tests must cover good frames, bad CRC/checksum, garbage before sync, and partial chunk assembly for both MAVLink and CRTP.
- Capability tests must verify MAVLink-to-capability mapping for attitude, battery, GPS, IMU, and system status.
- Runtime tests should preserve existing text parser behavior in auto mode.
- Cross-layer tests should assert new bus event kinds serialize through Tauri models without shape drift.
- Cross-layer tests should cover `ProtocolDetected` plus `TransportKind::Crazyradio -> UiTransportKind::Crazyradio` projection.
- Semantic mapping tests should assert the required `fields` keys for the covered MAVLink message IDs and CRTP ports/channels.
- Capability tests should assert CRTP commander RPYT produces `CapabilityEvent::Attitude`.
- CLI tests should assert `--parser mavlink`, `--parser crtp`, `--transport serial`, and `--transport crazyradio` parse successfully.
- Transport tests should verify `ByteTransport` trait implementations for serial and Crazyradio.

---

## 8. Wrong vs Correct

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
// Correct: transport and framing are independent concerns.
match config.transport {
    TransportSelection::Serial => {
        let port = open_serial_port(&config)?;
        let mut transport = SerialTransport::from_port(port, &config);
        run_framing_loop(&mut transport, parser)?;
    }
    TransportSelection::Crazyradio { uri } => {
        let mut transport = CrazyradioTransport::connect(&uri, datarate)?;
        run_framing_loop(&mut transport, parser)?;
    }
}
```
