# BMI088 UART4 Binary Telemetry Integration

## Goal
Integrate the BMI088 UART_4 binary schema-first telemetry protocol into the existing EADAI serial runtime so the app can automatically handshake, parse schema, stream samples, auto-create ordered variables, feed waveform/IMU views, and expose structured callbacks for scripting.

## Requirements
- Add host-side support for the documented BMI088 UART_4 binary frame format:
  - SOF `0xA5 0x5A`
  - version `0x01`
  - `REQUEST` / `EVENT` frame types
  - CRC16-CCITT (`0xFFFF`, poly `0x1021`, LE on wire)
- Parse and validate `SCHEMA` and `SAMPLE` frames according to the provided contract.
- Support host commands `ACK`, `START`, `STOP`, and `REQ_SCHEMA` as binary frames with empty payloads.
- Implement session flow:
  - boot `SCHEMA -> ACK -> START -> stream`
  - reconnect `REQ_SCHEMA -> ACK -> START`
  - `STOP -> START`
  - mid-stream `REQ_SCHEMA` must force re-ack + restart
- Add protocol auto-detection in the runtime so existing text telemetry still works while BMI088 binary frames can be recognized on the same serial connection path.
- Preserve ASCII compatibility commands outside binary frames (`ack`, `start`, `stop`, `escX=Y`).
- Surface structured schema/sample information across layers instead of relying only on text parsing.
- UI must auto-create 9 variables in exact schema order, apply `scale_q=-2` to angle fields, and keep IMU mapping/waveform features working.
- Add an application scripting callback API with these callbacks:
  - `onSchema(fields, rateHz, sampleLen)`
  - `onSample(record)`
- Keep MCP read-only, but make sure MCP can observe the same decoded telemetry through the shared adapter path.

## Acceptance Criteria
- [ ] Opening BMI088 UART_4 at `100000 8N1` can complete handshake and receive continuous `SAMPLE` frames.
- [ ] Parser validates CRC and can resynchronize after bad SOF, bad length, and bad CRC.
- [ ] Runtime auto-detects text vs BMI088 binary traffic without breaking the existing text path.
- [ ] UI auto-creates the 9 BMI088 variables in schema order and plots scaled angles correctly.
- [ ] IMU views can use the decoded BMI088 roll/pitch/yaw and raw accel/gyro data.
- [ ] Scripting callback API receives structured schema and sample payloads.
- [ ] MCP/AI adapter sees the same decoded telemetry state without introducing write access.
- [ ] Tests cover protocol parsing, handshake/session flow, reconnect behavior, and schema/sample decoding.

## Protocol Summary
- Baud / mode: `100000 8N1`
- Commands:
  - Host -> device: `ACK=0x10`, `START=0x11`, `STOP=0x12`, `REQ_SCHEMA=0x13`
  - Device -> host: `SCHEMA=0x80`, `SAMPLE=0x81`
- Sample payload contains 9 little-endian `i16` fields in schema order:
  - `acc_x`, `acc_y`, `acc_z`, `gyro_x`, `gyro_y`, `gyro_z`, `roll`, `pitch`, `yaw`
- Angle fields use `scale_q=-2` and unit `deg`.

## Constraints
- Do not remove or regress the existing fake-stream debug path.
- Prefer keeping protocol-specific logic out of Tauri/UI; decode once in Rust and fan out structured events.
- Preserve existing text telemetry support and current line-based analysis features where practical.
- Do not add MCP write tools.
