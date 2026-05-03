---
name: self-describing-protocol-integration
description: Use when implementing firmware for the repo's shared UART4 self-describing protocol path — the executable device-side contract for outer framing, identity, staged HostAck, paged catalogs, streaming samples, and UART ownership.
---

# Self-Describing Protocol Integration

这是设备侧协议手册，只定义 wire contract、状态机、编码规则和 UART 归属；不包含排障、日志检查表或诊断流程。

## 1) Scope

- 适用对象：`bmi088_protocol_*`、UART4 相关代码、Identity、HostAck、catalog、sample、SetVariable、AckResult。
- 目标：让 firmware 输出与 host 端期待的自描述协议完全一致。
- 约束：只描述协议本身，不描述如何诊断失败。

## 1.1) Portable reference pattern

- 如果要给设备侧写可移植参考实现，优先放在 `reference/` 目录下，保持为 application-layer C 代码。
- 所有 UART、锁、时钟、调试输出都必须通过驱动适配器函数指针注入，不直接依赖寄存器或 BSP。
- 参考实现应提供协议状态、Identity、catalog、sample 和 HostAck 处理，但不引入 host 兼容分支。

## 2) Executable wire contract

### Outer transport

- Outer transport is `header + len + payload`.
- No outer CRC.
- `header = 0x73`.
- Header comes from `BMI088_CRTP_PORT = 0x07` and `BMI088_CRTP_CHANNEL = 0x03`, via `(port << 4) | channel`.
- Ingress entry point: `bmi088_protocol_feed_byte()`.

### UART ownership

- `DEBUG_UART_INDEX = UART_4`
- `DEBUG_UART_TX_PIN = UART4_TX_P19_1`
- `DEBUG_UART_RX_PIN = UART4_RX_P19_0`
- This UART carries protocol frames only.

### Identity frame

- Boot first frame is `Identity`.
- Emitted by `bmi088_uart4_telemetry_init()` via `bmi088_protocol_send_identity()`.
- Identity payload:
  - `type = 0x01`
  - `protocol_version`
  - `device_name`
  - `firmware_version`
  - `sample_rate_hz (u32 LE)`
  - `variable_count (u16 LE)`
  - `command_count (u16 LE)`
  - `sample_payload_len (u16 LE)`

### HostAck contract

- HostAck payload is exactly two bytes:
  - `payload[0] = 0x04`
  - `payload[1] = stage`
- Valid stages:
  - `0x01` Identity
  - `0x02` Command
  - `0x03` Variable
- ACK advances only when the current state matches:
  - `IDENTITY_SENT + 0x01`
  - `COMMAND_SENT + 0x02`
  - `WAIT_HOST_ACK + 0x03`

### Handshake state machine

- `IdentitySent -> CommandSent -> WaitHostAck -> Streaming`
- Identity stage emits `HostAck(stage=0x01)`.
- After `Identity -> HostAck(Identity)`, command catalog and variable catalog must be emitted as canonical self-describing frames directly.
- Do not add a private `F3` catalog fragment layer on the target path.
- Do not keep transitional `u8/u8/u8` page headers when the host expects canonical page framing.
- If `F3` wrappers or transitional headers appear on the wire, treat that as device-side contract drift and fix the device, not the host.
- Command catalog stage emits `HostAck(stage=0x02)`.
- Variable catalog stage emits `HostAck(stage=0x03)`.
- ACK only advances the current stage; it does not skip or compensate for later stages.

### Command catalog contract

- Command catalog page limit = 16.
- Catalog pages are ordered and finite; page encoding must use the host's canonical page framing.
- Command catalog payloads remain protocol frames, not text summaries.
- Command catalog pages are canonical self-describing frames, not `F3` fragments.

### Variable catalog contract

- Variable catalog page limit = 32.
- Variable pages follow the same transport and framing rules as command pages.
- Variable catalog payloads remain protocol frames, not text summaries.
- Variable catalog pages are canonical self-describing frames, not `F3` fragments.

### SetVariable / AckResult contract

- `SetVariable` uses the typed protocol path, not the legacy shell path.
- `AckResult` is the device-side typed acknowledgement for variable writeback.
- `SetVariable` and `AckResult` are part of the real protocol path; the legacy shell path is effectively inactive.

### Sample contract

- Sample type is `0x05`.
- Sample payload is bitmap-compressed:
  - `seq (u32 LE)`
  - `bitmap_len (u16 LE)`
  - `bitmap`
  - changed values in variable order
- Bitmap bit order matches variable order.
- Values are serialized as raw little-endian bytes for the changed variables only.

## 3) Firmware entry points

- `bmi088_uart4_telemetry_init()` initializes UART4 and sends Identity.
- `bmi088_protocol_feed_byte()` is the only byte ingress.
- `bmi088_protocol_send_identity()` encodes Identity.
- `bmi088_protocol_handle_host_ack()` validates HostAck and advances state.
- `bmi088_protocol_send_command_catalog_page()` sends one command page.
- `bmi088_protocol_send_variable_catalog_page()` sends one variable page.
- `bmi088_protocol_enter_streaming()` enters streaming.

## 4) Non-negotiable invariants

1. HostAck is exactly two bytes: `0x04 + stage`.
2. Outer transport has no CRC.
3. UART4 carries protocol frames only.
4. Identity is the boot first frame.
5. ACK only advances the matching current state.
6. Catalog pages use fixed per-page limits: command 16, variable 32.
7. Catalog pages must use canonical host framing; `F3` wrappers and transitional `u8/u8/u8` headers are contract drift.
8. Samples use bitmap compression with `type = 0x05`.
9. Legacy shell is inactive; typed protocol is the real path.

## 5) Wrong vs Correct

### Wrong

```c
// Wrong: add extra bytes to HostAck.
// Wrong: treat UART4 as a debug text sink.
```

### Correct

```c
// Correct: HostAck is exactly 0x04 + stage, and UART4 carries protocol frames only.
```

## 6) Copy template

```text
Goal: implement the device-side self-describing protocol on UART4.
First ensure: outer framing, Identity, HostAck, catalog paging, sample bitmap encoding, and UART ownership.
```
