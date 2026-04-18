---
name: bmi088-host-handshake-sync
description: Use when implementing or debugging a host-side serial parser, handshake state machine, or schema-driven BMI088 telemetry client for the shared UART4/P19 debug port.
---

# BMI088 Host Handshake Sync

当 Agent 需要开发、联调或排查上位机对 `BMI088 UART_4` 遥测链路的握手同步功能时，优先加载这个 skill，而不是把这条串口当普通文本日志口处理。

## When to Use
- 需要给上位机实现 `SCHEMA -> ACK -> START -> SAMPLE` 握手流程。
- 需要写串口解析器、CRC 校验器、schema 注册器或 sample 解码器。
- 需要排查“上电只看到乱码/字段名后没数据”“发了 start 也没流”“串口看起来像卡死”这类问题。
- 需要指导另一个项目或 Agent 对接当前仓库里 `project/user/bmi088_uart4_telemetry.c` 的线协议。

## Runtime Wiring
- UART instance: `UART_4`
- Baud: `115200`
- TX pin: `UART4_TX_P19_1`
- RX pin: `UART4_RX_P19_0`
- Mode: `8N1`
- Port ownership:
  - 这是板上唯一有效的 debug/telemetry 共用口。
  - 不要把 `SCB2/UART_4` 再重映射到 `P14_1/P14_0`。

## Firmware Entry Points
- `project/user/bmi088_uart4_telemetry.c`
  - `bmi088_uart4_telemetry_init(void)`
  - `bmi088_uart4_telemetry_poll(void)`
  - `bmi088_uart4_telemetry_push_sample(void)`
- `project/user/main_cm7_0.c`
  - 主循环每 `1 ms` 调一次 `bmi088_uart4_telemetry_poll()`
  - BMI088 数据每 `10 ms` 更新一次
  - 仅在 `STREAMING` 状态下才会发送 `SAMPLE`

## Protocol Summary

### Frame Envelope

| Field | Size | Value / Meaning |
|---|---:|---|
| `sof1` | 1 | `0xA5` |
| `sof2` | 1 | `0x5A` |
| `version` | 1 | `0x01` |
| `type` | 1 | request / response / event |
| `cmd` | 1 | command id |
| `seq` | 1 | transmit sequence |
| `len` | 1 | payload length |
| `payload` | `len` | variable data |
| `crc16` | 2 | CRC16-CCITT, little-endian on wire |

CRC contract:
- init: `0xFFFF`
- polynomial: `0x1021`
- input bytes: frame header + payload only
- wire order: low byte first, high byte second

### Frame Types

| Name | Value | Direction |
|---|---:|---|
| `REQUEST` | `0x01` | host -> device |
| `RESPONSE` | `0x02` | reserved |
| `EVENT` | `0x03` | device -> host |

### Commands

| Name | Value | Direction | Meaning |
|---|---:|---|---|
| `ACK` | `0x10` | host -> device | schema accepted |
| `START` | `0x11` | host -> device | start streaming |
| `STOP` | `0x12` | host -> device | stop streaming |
| `REQ_SCHEMA` | `0x13` | host -> device | request schema resend |
| `SCHEMA` | `0x80` | device -> host | ordered field metadata |
| `SAMPLE` | `0x81` | device -> host | one BMI088 sample |

## Handshake State Machine

### Device-side States
- `WAIT_ACK`
- `WAIT_START`
- `STREAMING`

### Current Boot / Recovery Behavior
- Power-on:
  - initialize UART and parser
  - enter `WAIT_ACK`
  - send one `SCHEMA` immediately
- Retry policy while not streaming:
  - in `WAIT_ACK` or `WAIT_START`, firmware resends `SCHEMA` every `500 ms`
  - this is driven from `bmi088_uart4_telemetry_poll()`
- Transition rules:
  - `WAIT_ACK` + `ACK` -> `WAIT_START`
  - `WAIT_START` + `START` -> `STREAMING`
  - `STOP` -> `WAIT_START`
  - `REQ_SCHEMA` -> resend schema, then `WAIT_ACK`

### Practical Meaning
- “上电后先出现一串乱码，随后安静”通常不是死机，而是：
  - 你在 ASCII 终端里看到了二进制 `SCHEMA`
  - 设备正在等 `ACK`
- 如果握手没完成，`bmi088_uart4_telemetry_push_sample()` 会直接返回，不会发 `SAMPLE`

## Schema Contract

### Top-level Payload

| Field | Size | Meaning |
|---|---:|---|
| `schema_version` | 1 | fixed `0x01` |
| `sample_rate_hz` | 1 | fixed `100` |
| `field_count` | 1 | fixed `9` |
| `sample_len` | 1 | fixed `18` |
| `field_desc[]` | var | repeated field descriptors |

### Field Descriptor Layout

| Field | Size | Meaning |
|---|---:|---|
| `field_id` | 1 | zero-based id |
| `field_type` | 1 | currently `1 = i16` |
| `scale_q` | 1 | decimal exponent |
| `name_len` | 1 | ASCII field name length |
| `unit_len` | 1 | ASCII unit length |
| `name` | var | no terminator |
| `unit` | var | no terminator |

### Current Ordered Fields

| ID | Name | Type | `scale_q` | Unit |
|---|---|---|---:|---|
| 0 | `acc_x` | `i16` | 0 | `raw` |
| 1 | `acc_y` | `i16` | 0 | `raw` |
| 2 | `acc_z` | `i16` | 0 | `raw` |
| 3 | `gyro_x` | `i16` | 0 | `raw` |
| 4 | `gyro_y` | `i16` | 0 | `raw` |
| 5 | `gyro_z` | `i16` | 0 | `raw` |
| 6 | `roll` | `i16` | -2 | `deg` |
| 7 | `pitch` | `i16` | -2 | `deg` |
| 8 | `yaw` | `i16` | -2 | `deg` |

Host rule:
- 永远按 `SCHEMA` 建表，不要把 `SAMPLE` 直接硬编码成固定 C struct。

## Sample Contract
- payload length fixed: `18` bytes
- layout: 9 x `int16 little-endian`
- order exactly matches schema order
- angle conversion:
  - `roll/pitch/yaw` physical value = `raw / 100.0`

## Host Implementation Pattern

### Recommended Modules
- serial reader
- byte-stream frame parser
- session state machine
- schema registry
- sample decoder
- UI / plotting layer

### Recommended Flow
1. Open serial at `115200 8N1`.
2. Wait for boot `SCHEMA` or receive a periodic schema resend.
3. Parse the frame envelope first.
4. Parse schema and register all fields by name/order.
5. Send `ACK`.
6. Send `START`.
7. Decode each `SAMPLE` using the schema-derived order.
8. On reconnect, desync, or bad parser state, send `REQ_SCHEMA` and restart the handshake.

### Minimal Pseudocode
```text
open_serial(baud=115200, data_bits=8, parity='N', stop_bits=1)

schema = wait_for_schema(timeout_ms=1000)
if schema is None:
    send_frame(type=0x01, cmd=0x13, payload=[])
    schema = wait_for_schema(timeout_ms=1000)

register_fields(schema)
send_frame(type=0x01, cmd=0x10, payload=[])
send_frame(type=0x01, cmd=0x11, payload=[])

while port_open:
    frame = read_next_valid_frame()
    if frame.cmd == 0x80:
        schema = parse_schema(frame.payload)
        register_fields(schema)
        send_frame(type=0x01, cmd=0x10, payload=[])
        send_frame(type=0x01, cmd=0x11, payload=[])
    elif frame.cmd == 0x81:
        sample = decode_sample(frame.payload, schema)
        publish(sample)
```

## ASCII Compatibility
- The same RX path also accepts printable ASCII commands outside binary frames.
- Supported ASCII commands:
  - `ack`
  - `start`
  - `stop`
  - `escX=Y`
- This is useful for quick manual bring-up with a serial assistant, but a real host tool should prefer binary frames.

## Common Pitfalls
- 把 `UART_4` 当成纯文本日志口，用 ASCII 助手直接看二进制帧后误判“乱码/死机”。
- 串口参数开成 `100000`，而不是当前固件实际使用的 `115200`。
- 发送 `START` 之前没先完成 `ACK`。
- 在上位机里把 `SAMPLE` 当 float 数组而不是 `9 x i16 LE`。
- 看到一次 `SCHEMA` 后没等重发，也没主动发 `REQ_SCHEMA`。
- 在同一条线里插入任意文本调试输出，破坏二进制帧边界。

## Troubleshooting Checklist

### Symptom: only one garbled burst after boot
- Most likely: terminal is displaying the binary `SCHEMA` frame.
- Action:
  - switch tool to raw serial handling
  - confirm `115200 8N1`
  - send `ACK`, then `START`

### Symptom: no samples after schema
- Check host really sent `ACK` first, then `START`.
- Check outgoing frame CRC is correct.
- Check host is writing to `P19.0/P19.1` shared UART, not an old `P14` path.

### Symptom: schema keeps repeating every 500 ms
- This means firmware is alive but still not satisfied with handshake.
- Most likely causes:
  - host command never reached MCU
  - CRC wrong
  - wrong baud
  - host sent printable text without newline when expecting ASCII path

### Symptom: parser loses sync mid-stream
- Drop current partial frame.
- Scan for next `0xA5 0x5A`.
- Optionally send `REQ_SCHEMA` and restart session setup.

## Verification Checklist
- Boot receives `SCHEMA` within `0.5 s` and then periodic resends until handshake completes.
- Host can parse all 9 fields and preserve their schema order.
- `ACK -> START` switches stream into continuous `SAMPLE` output.
- `STOP` halts samples without requiring reboot.
- `REQ_SCHEMA` during stream forces schema resend and re-handshake.

## Agent Instructions
- When changing the host, keep handshake/session logic separate from frame parsing.
- Prefer schema-driven decode over fixed offsets hard-coded in UI code.
- If debugging “freeze”, first test whether schema is simply being retransmitted and the device is waiting for `ACK`.
- If editing firmware and adding new `.c`/`.h`, remember IAR `.ewp` file updates may also be required.

## Copy Template
```text
Goal: implement a host-side handshake and stream decoder for the BMI088 shared debug/telemetry UART.

Constraints:
- Use UART_4 on P19.1/P19.0 at 115200 8N1.
- Treat the line as a binary protocol port, not a plain text debug console.
- Parse SCHEMA first, then send ACK, then START.
- Decode SAMPLE as 9 little-endian int16 values using schema order.
- If no schema is seen after connect, send REQ_SCHEMA and retry.
- Be tolerant of reconnects, bad CRC, and parser desync.

Implementation hints:
- Separate serial read, frame parsing, session state, schema registry, and sample decode.
- Resync on SOF 0xA5 0x5A.
- Use CRC16-CCITT with init 0xFFFF and polynomial 0x1021.
- Support periodic schema resends before streaming starts.
```
