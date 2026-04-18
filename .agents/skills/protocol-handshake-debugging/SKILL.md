---
name: protocol-handshake-debugging
description: Debug schema-first binary serial protocols, handshake state machines, and mixed text/binary streams. Use when host assumptions may be stale and the executable wire contract must be verified with a minimal diagnostic path.
---

# Protocol Handshake Debugging

Use this skill when debugging schema-first binary serial links, mixed text/binary streams, reconnect-sensitive handshakes, or high-rate telemetry pipelines where the host/device contract may have drifted.

## When to Use
- 新协议接入后出现“能连口但没数据”“有乱码但 UI 没流”。
- 上位机 handshake 已实现，但 `ACK/START/READY` 类流程始终卡住。
- 高速二进制流偶发解码失败、重连后失步、只在冷启动或热重连其中一种情况下工作。
- 需要为某个协议编写最小诊断工具，先验证线协议再回填主应用。

## Repo Touchpoints
- `src/app.rs`: runtime 生命周期、重连、自动发命令逻辑。
- `src/serial.rs`: 原始字节读取、transport 边界。
- `src/message.rs`: 结构化事件总线，不要把原始字节直接泄漏到 UI 语义层。
- `src/bin/bmi088-handshake-diag.rs`: 最小握手诊断 CLI 模式示例。
- `tests/`: 协议回归、重同步、握手状态机测试都应放这里。

## Root Cause Taxonomy
先分类，不要上来就重写 parser：

| Layer | Typical failures |
|---|---|
| Transport | 错串口、错波特率、超时/换行/缓冲配置不一致 |
| Framing | SOF、长度、CRC、转义、半包处理错误 |
| Session | handshake 顺序错误、重连后状态未复位、设备仍在等前置命令 |
| Schema | host 仍按旧字段布局/旧版本解码 |
| Decode | endian、signedness、scale、字段顺序错误 |
| UI contract | 后端已解析，但 message/store/UI 绑定错层 |

## Correct Architecture Split
把职责切开：
- **serial reader**: 只负责拿原始字节。
- **frame parser**: 只负责切帧、校验、重同步。
- **session state**: 只负责 `WAIT_SCHEMA -> ACK -> START -> STREAMING` 这类状态迁移。
- **schema registry**: 记录当前字段顺序/类型/缩放。
- **sample decoder**: 在 schema 驱动下把 payload 变成 typed sample。
- **UI/store**: 只消费 `src/message.rs` 里的结构化事件。

如果 UI 在猜 payload offset，或者 parser 在偷偷维护业务状态，后面一定会难排障。

## Minimal Diagnostic Strategy
当怀疑 host 假设已过时时，先走最小诊断路径：
1. 打开串口，只读原始字节。
2. 记录有限长度 hex preview，不打印整段高速流。
3. 先验证设备是否周期发送 boot/schema/heartbeat。
4. 用最小二进制命令逐个触发状态迁移。
5. 若协议允许，再用 ASCII fallback 验证 RX 通路是否物理可达。
6. 只有在线协议被最小工具验证后，才回头改主 runtime/UI。

**Transferable lesson:** 不要默认 host 里的旧协议实现是对的；先验证“当前线上设备真正说的是什么”。

## Logging Rules
- 在 transport 边界记录 `chunk size / preview / total bytes`。
- 在协议边界记录 `frame type / cmd / seq / len / crc result`。
- 在 session 边界记录状态迁移，不刷高速 payload。
- 在 UI 边界记录“收到了多少 schema/sample 事件”，不要回退到看原始二进制猜问题。
- 原始日志必须有上限；高频流不要全量 dump。

## Verification Checklist
- 能在冷启动下看到首个控制帧/boot schema。
- 能在重连后重新握手，而不是只在第一次连接成功。
- 坏 CRC、坏长度、坏 SOF 时，解析器能重新找回帧边界。
- sample decode 使用最新 schema，而不是硬编码旧字段顺序。
- `cargo test` 中有 focused protocol tests，而不是只靠手工串口观察。

## Anti-Patterns
- 直接在 UI 层解析二进制 payload。
- 文本协议和二进制协议共用一个混乱状态机。
- handshake 失败时，只加更多 println，而不区分 transport/framing/session。
- 没有最小诊断工具，就直接改主应用大逻辑。
- 设备协议升级后，host 还偷偷复用旧字段布局。

## Recommended Deliverables
做这类任务时，优先产出：
- 一个最小诊断命令或 binary
- 一组 focused protocol tests
- 一个明确的 verdict 规则（例如 `schema repeating` / `no valid frames` / `samples flowing`）
- 一个项目 skill 或文档更新，避免同类错误重复出现
