---
name: self-describing-protocol-integration
description: Use when implementing or debugging firmware that adopts the generic self-describing device protocol — identity declaration, command/variable catalog handshake, bitmap-compressed telemetry streaming, and host variable write-back.
---

# Self-Describing Protocol Integration

当固件工程师需要让新设备接入 EADAI 的通用自描述协议时，优先加载这个 skill。它覆盖从身份声明到变量回写的完整设备侧实现路径，而不是在 host-side 调试或 CRTP 专用适配上。

## When to Use

- 需要为新设备实现通用自描述协议握手（身份、命令目录、变量目录）。
- 需要实现带位图压缩的 telemetry sample 编码。
- 需要处理 host 发来的 `SetVariable` 回写请求。
- 需要理解设备侧状态机应该如何驱动各阶段转换。
- 需要排查"host 看不到 identity""目录没传完就卡住""sample 丢了某些字段"等问题。

## Device-Side Minimal Pseudocode

以下是语言中立的设备侧实现骨架，覆盖从上电到 streaming 的完整流程：

```
// --- State ---
enum State { IDENTITY_SENT, CMD_CATALOG_SENT, VAR_CATALOG_SENT, WAIT_HOST_ACK, STREAMING }
state = IDENTITY_SENT

// --- Setup (called once at boot) ---
vars = [Variable{ name:"acc_x", type:i16, order:0, unit:"raw", adjustable:false },
        Variable{ name:"acc_y", type:i16, order:1, unit:"raw", adjustable:false },
        Variable{ name:"gain",  type:f32, order:2, unit:"x",  adjustable:true  }]
cmds = [Command{ id:"reset", params:"", docs:"Reset device" }]

sample_buf = allocate_bytes(sum_of(v.type.byte_size for v in vars))
prev_values = copy(sample_buf)

send_identity(protocol_version=1, device_name="SensorX", fw_version="1.0",
              sample_rate=100, var_count=len(vars), cmd_count=len(cmds),
              sample_payload_len=sum_of(v.type.byte_size))

total_cmd_pages = ceil(len(cmds) / 16)
for page in 0..total_cmd_pages:
    send_cmd_catalog_page(page, total_cmd_pages, cmds[page*16 : (page+1)*16])

total_var_pages = ceil(len(vars) / 32)
for page in 0..total_var_pages:
    send_var_catalog_page(page, total_var_pages, vars[page*32 : (page+1)*32])

state = CMD_CATALOG_SENT  // then VAR_CATALOG_SENT after last page

// --- Main loop ---
while running:
    if state == WAIT_HOST_ACK:
        frame = try_read_frame(timeout_ms=10)
        if frame.type == HOST_ACK:
            state = STREAMING

    elif state == STREAMING:
        read_sensors(sample_buf)

        // Build bitmap: compare each var with prev_values
        bitmap = 0
        changed = []
        for i, var in enumerate(vars):
            offset = var.order * var.type.byte_size
            if sample_buf[offset..] != prev_values[offset..]:
                bitmap |= (1 << i)
                changed.append(var)

        send_telemetry_sample(seq=next_seq(), bitmap=bitmap,
                              values=[sample_buf[v.order * v.type.byte_size ..]
                                      for v in changed])

        copy(sample_buf, prev_values)

    // Handle SetVariable (polled or interrupt-driven)
    frame = try_read_frame(timeout_ms=0)
    if frame.type == SET_VARIABLE:
        result = handle_set_variable(frame)
        send_ack_result(frame.seq, result.code, result.message)

// --- SetVariable handler ---
function handle_set_variable(frame):
    idx = frame.variable_index
    if idx >= len(vars):
        return { code: 0x01, message: "InvalidIndex" }
    if not vars[idx].adjustable:
        return { code: 0x03, message: "ReadOnly" }
    if not in_range(vars[idx], frame.value):
        return { code: 0x02, message: "OutOfRange" }

    apply_value(vars[idx], frame.value)
    return { code: 0x00, message: "" }
```

**关键点**：
- 状态机与 transport 解耦；`send_*` / `try_read_frame` 由 adapter 提供。
- 首帧 bitmap 全 1，后续帧只标记与 `prev_values` 不同的变量。
- `SetVariable` 处理器必须检查 index、adjustable、range 三道关。

## Protocol Overview

- **协议层级**：transport-agnostic。协议本身不绑定 CRTP、serial 或任何特定链路。
- **一期承载**：CRTP over serial，debug port (0x7)，channel 3。
- **设计原则**：状态机和数据模型不写死在 CRTP port/channel 语义里；换 transport 只需换 adapter。

## Handshake Phases

设备上电后必须按以下顺序向主机声明自身能力：

```
1. Identity           (device -> host)
2. CommandCatalog     (device -> host, may be paged)
3. VariableCatalog    (device -> host, may be paged)
4. HostAck            (host -> device, stage-level ACK)
5. Streaming starts   (device -> host, TelemetrySample frames)
```

**设备侧状态机**：
```
IDENTITY_SENT -> CMD_CATALOG_SENT -> VAR_CATALOG_SENT
    -> WAIT_HOST_ACK -> STREAMING
```

设备在每个阶段完成后进入等待状态，直到收到 host 的 `HostAck` 才推进到下一阶段。

## Frame Types

| Frame | 方向 | 类型码 | 用途 |
|-------|------|--------|------|
| `Identity` | device → host | `0x01` | 设备身份、协议版本、采样率、变量/命令数量 |
| `CommandCatalogPage` | device → host | `0x03` | 命令目录分页 |
| `VariableCatalogPage` | device → host | `0x02` | 变量目录分页 |
| `HostAck` | host → device | `0x04` | 主机对阶段完成的确认 |
| `TelemetrySample` | device → host | `0x05` | 遥测数据帧（带位图压缩） |
| `SetVariable` | host → device | `0x06` | 主机写入可调变量 |
| `AckResult` | device → host | `0x07` | 设备对 SetVariable 的确认 |

## Identity Frame

设备上电后首先发送 `Identity`，声明以下信息：

| 字段 | 类型 | 说明 |
|------|------|------|
| `protocol_version` | u8 | 协议版本，当前为 `1` |
| `device_name` | string | 设备名称（UTF-8） |
| `firmware_version` | string | 固件版本（UTF-8） |
| `sample_rate_hz` | u32 | 遥测采样率 |
| `variable_count` | u16 | 变量目录总条目数 |
| `command_count` | u16 | 命令目录总条目数 |
| `sample_payload_len` | u16 | 单个 sample 的完整 payload 字节数 |

**关键约束**：`sample_payload_len` 必须等于所有变量 `value_type.byte_size()` 之和。host 用此字段验证目录一致性。

## Command Catalog

命令目录允许分页传输，每页最多 16 条命令。

每条命令描述符包含：
- `id`（string）：命令标识
- `params`（string）：参数描述
- `docs`（string）：基础文档文本

分页字段：`page`（0-based）、`total_pages`。

**设备侧要求**：
- 所有 `total_pages` 必须一致。
- 页码从 0 开始连续递增。
- 无命令时仍需发送一页（`total_pages=1`, `count=0`）。

## Variable Catalog

变量目录允许分页传输，每页最多 32 条变量。

每条变量描述符包含：
- `name`（string）：变量名（UTF-8）
- `order`（u16）：sample payload 中的字节顺序（0-based）
- `unit`（string）：单位（UTF-8）
- `adjustable`（bool）：是否允许 host 写回
- `value_type`（u8）：值编码类型

**值类型编码**：

| 类型码 | 类型 | 字节大小 |
|--------|------|----------|
| `0x01` | u8 | 1 |
| `0x02` | i8 | 1 |
| `0x03` | u16 | 2 |
| `0x04` | i16 | 2 |
| `0x05` | u32 | 4 |
| `0x06` | i32 | 4 |
| `0x07` | f32 | 4 |

**设备侧要求**：
- `order` 字段必须与 sample payload 中变量的字节偏移一致。
- 所有变量的字节大小之和必须等于 `Identity.sample_payload_len`。
- 变量顺序一旦确定不可动态重排（一期约束）。

## TelemetrySample Encoding

Sample 帧使用位图压缩，只传输值发生变化的变量。

### 帧结构

| 字段 | 大小 | 说明 |
|------|------|------|
| `seq` | 4 bytes (u32 LE) | 单调递增的序列号 |
| `bitmap_len` | 2 bytes (u16 LE) | 位图字节数 |
| `changed_bitmap` | `bitmap_len` bytes | 位图，bit i = 1 表示第 i 个变量有变化 |
| `values` | variable | 仅包含有变化的变量值，按 order 顺序紧密排列 |

### 编码规则

1. 位图字节数 = `ceil(variable_count / 8)`。
2. 第一个 sample 所有变量标记为已变化（bitmap 全 1）。
3. 后续 sample 只标记值与上一次不同的变量。
4. `values` 字段仅包含已变化变量的原始字节，按变量 `order` 顺序排列。
5. 所有多字节数值采用 **little-endian** 编码。

### 解码示例

假设有 3 个变量：`acc_x(i16)`, `acc_y(i16)`, `gain(f32)`。

第一个 sample：bitmap = `0b00000111`（全部变化），values = `[acc_x_lo, acc_x_hi, acc_y_lo, acc_y_hi, gain_0..3]`（8 bytes）。

第二个 sample：仅 `acc_x` 变化，bitmap = `0b00000001`，values = `[acc_x_lo, acc_x_hi]`（2 bytes，跳过 acc_y 和 gain）。

### 关键帧示例（hex）

以下示例使用 3 变量设备：`acc_x(i16, order=0)`, `acc_y(i16, order=1)`, `gain(f32, order=2)`。变量字节大小之和 = 2+2+4 = 8。

**Identity 帧** (`0x01`)：

```
01                          -- frame type
01                          -- protocol_version = 1
00 06                       -- device_name len = 6 ("SensorX")
53 65 6E 73 6F 72           -- "SensorX"
00 03                       -- firmware_version len = 3 ("1.0")
31 2E 30                    -- "1.0"
64 00 00 00                 -- sample_rate_hz = 100
03 00                       -- variable_count = 3
01 00                       -- command_count = 1
08 00                       -- sample_payload_len = 8
```

**VariableCatalogPage** (`0x02`)：

```
02                          -- frame type
00                          -- page = 0
01                          -- total_pages = 1
03                          -- count = 3
-- var 0: acc_x
00 06                       -- name len = 6
61 63 63 5F 78              -- "acc_x"
00 00                       -- order = 0
00 01                       -- unit len = 1
72                          -- "r" (raw)
00                          -- adjustable = false
04                          -- value_type = i16 (0x04)
-- var 1: acc_y
00 06                       -- name len = 6
61 63 63 5F 79              -- "acc_y"
01 00                       -- order = 1
00 01                       -- unit len = 1
72                          -- "r"
00                          -- adjustable = false
04                          -- value_type = i16
-- var 2: gain
00 04                       -- name len = 4
67 61 69 6E                 -- "gain"
02 00                       -- order = 2
00 01                       -- unit len = 1
78                          -- "x" (multiplier)
01                          -- adjustable = true
07                          -- value_type = f32 (0x07)
```

**TelemetrySample — 首帧**（全变量变化，bitmap 全 1）：

```
05                          -- frame type
01 00 00 00                 -- seq = 1
01 00                       -- bitmap_len = 1
07                          -- bitmap = 0b00000111 (3 vars all changed)
E8 03                       -- acc_x = 1000 (i16 LE)
D0 07                       -- acc_y = 2000 (i16 LE)
00 00 80 3F                 -- gain = 1.0 (f32 LE, 0x3F800000)
```

**TelemetrySample — 后续帧**（仅 acc_x 变化）：

```
05                          -- frame type
02 00 00 00                 -- seq = 2
01 00                       -- bitmap_len = 1
01                          -- bitmap = 0b00000001 (only var 0 changed)
EC 03                       -- acc_x = 1004 (i16 LE)
```

**SetVariable 请求**（写入 gain = 2.5）：

```
06                          -- frame type
0A 00 00 00                 -- seq = 10
02 00                       -- variable_index = 2 (gain)
04 00                       -- value_len = 4
00 00 20 40                 -- gain = 2.5 (f32 LE, 0x40200000)
```

**AckResult 响应**（成功）：

```
07                          -- frame type
0A 00 00 00                 -- seq = 10 (matches request)
00                          -- code = 0 (success)
00 00                       -- message len = 0
```

## SetVariable Write-Back

主机可以对 `adjustable=true` 的变量发送 `SetVariable` 帧。

### SetVariable 帧结构

| 字段 | 大小 | 说明 |
|------|------|------|
| `seq` | 4 bytes (u32 LE) | 请求序列号，用于匹配响应 |
| `variable_index` | 2 bytes (u16 LE) | 变量索引（0-based） |
| `value_len` | 2 bytes (u16 LE) | 值字节数 |
| `value` | `value_len` bytes | 原始值字节（little-endian） |

### 设备侧处理

1. 收到 `SetVariable` 后，设备必须回复 `AckResult`。
2. `AckResult` 的 `seq` 必须与请求的 `seq` 一致。
3. `code = 0` 表示成功，非零表示错误。
4. 主机在收到成功确认后才让新值生效。

### AckResult 帧结构

| 字段 | 大小 | 说明 |
|------|------|------|
| `seq` | 4 bytes (u32 LE) | 对应请求的序列号 |
| `code` | 1 byte | 0 = success, 非零 = error |
| `message` | string | 可选的错误描述 |

### 推荐错误码约定

| Code | 含义 | 触发场景 |
|------|------|---------|
| `0x00` | Success | 写入成功 |
| `0x01` | InvalidIndex | `variable_index` 越界 |
| `0x02` | OutOfRange | 值超出允许范围 |
| `0x03` | ReadOnly | 变量 `adjustable=false` |
| `0x04` | BadCRC | 帧校验失败 |
| `0x05` | Busy | 设备暂时无法处理（如正在校准） |

设备侧应优先使用这些约定码，便于 host 侧统一错误处理逻辑。

## Wire Format (Codec)

所有帧的编码格式：

- 第一个字节为 **frame type** 标识符。
- 后续字节为帧载荷。
- 字符串编码：`u16 LE length` + `UTF-8 bytes`（无 null 终止符）。
- 数值编码：全部 little-endian。

**注意**：本协议不包含 SOF/CRC 帧信封。帧信封由底层 transport 提供（如 CRTP 自带 CRC-8 和地址信息）。如果 transport 层没有帧信封，固件需要自行添加。

## CRTP Transport (一期)

一期使用 CRTP over serial 承载本协议：

| 参数 | 值 |
|------|-----|
| CRTP Port | `0x07` (Debug) |
| CRTP Channel | `3` |
| 方向 | TX/RX over serial link |

CRTP adapter 位于 `src/protocols/self_describing/crtp_adapter.rs`。

**重要**：协议模型本身不绑定 CRTP。CRTP 只是 adapter 层。如果未来需要换 transport（如 USB native），只需实现新的 adapter。

## Common Pitfalls

- **忘记发 Identity**：host 等不到 identity 就不会推进握手，表现为"连上了但没反应"。
- **total_pages 不一致**：同一目录的所有分页必须声明相同的 `total_pages`，否则 host 报错。
- **order 与 payload 不匹配**：变量 `order` 必须与 sample 中的字节偏移精确对应，否则所有值都会解错。
- **bitmap 位数不对**：bitmap 字节数必须是 `ceil(variable_count / 8)`，多出的高位必须为 0。
- **小端序搞错**：所有多字节数值必须 little-endian，用 `to_le_bytes()` / `from_le_bytes()`。
- **首个 sample 位图未全 1**：第一次发送 sample 时，所有变量都应标记为已变化。
- **SetVariable 索引越界**：`variable_index` 必须在 `[0, variable_count)` 范围内。
- **未等 HostAck 就发 sample**：设备必须等到 host 确认变量目录后才能开始 streaming。
- **在协议帧里加 SOF/CRC**：CRTP 已提供帧信封，不要重复添加；如果换 transport 需要评估。
- **把协议硬编码在 CRTP 上**：状态机和数据模型必须 transport-agnostic，只在 adapter 层引用 CRTP。

## Troubleshooting

| 现象 | 可能原因 | 排查方向 |
|------|---------|---------|
| Host 连上后无反应 | Identity 未发送或 CRTP port/channel 错 | 抓包确认首帧是否为 `0x01` identity |
| 握手卡在 variable catalog | `total_pages` 不一致或 `order` 与 payload 不匹配 | 检查每页的 `total_pages` 字段，核对变量字节偏移 |
| Sample 只出现一次就不动 | 设备未等 HostAck 就切换到 streaming 状态 | 状态机是否在 `WAIT_HOST_ACK` 阻塞 |
| SetVariable 永远返回 error | `variable_index` 越界或变量非 `adjustable` | 检查 index 范围和 `adjustable` 标志 |
| Host 端值全错 | 小端序搞反或 bitmap 位号与 order 错位 | 用 hex dump 对照 frame examples 逐字节验证 |

## Verification Checklist

- [ ] 设备上电后在 500ms 内发出 `Identity`。
- [ ] `Identity.sample_payload_len` 等于所有变量字节大小之和。
- [ ] 命令目录分页正确，`total_pages` 一致，无重复页码。
- [ ] 变量目录分页正确，`order` 与 sample payload 偏移一致。
- [ ] 收到 `HostAck(VariableCatalog)` 后才开始发送 sample。
- [ ] 第一个 sample 的 bitmap 全为 1。
- [ ] 后续 sample 只标记值变化的变量。
- [ ] 多字节数值全部 little-endian。
- [ ] `SetVariable` 收到后回复 `AckResult`，`seq` 匹配。
- [ ] `SetVariable` 索引越界时返回错误 code。
- [ ] 协议状态机不引用 CRTP port/channel 语义（transport-agnostic）。

## Agent Instructions

- 实现设备侧协议时，先定义所有帧类型的编码/解码，再写状态机，最后写 sample 编码。
- 状态机和数据模型放在 transport-agnostic 的模块里，CRTP 适配单独放 adapter 模块。
- 测试覆盖：identity roundtrip、多页目录、bitmap 压缩/解压、set variable ack、错误状态。
- 如果需要新增 value type，同步更新 `ValueType` 枚举和 codec 的 encode/decode。
- 不要在 UI 或 Tauri 层做二进制协议解析；所有 decode 逻辑留在 Rust 后端。
