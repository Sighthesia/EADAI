# 内置 Crazyflie CRTP 与 MAVLink 协议支持

## Goal

为 EADAI 增加开箱即用的 Crazyflie CRTP 与 MAVLink 协议识别与解析能力，让串口接入后端可以自动识别协议、输出统一总线事件，并为后续 UI 展示与控制扩展保留清晰的协议边界。

## What I already know

* 当前运行时主链路在 `src/app/mod.rs`：串口读字节后，文本走 `parser::parse_framed_line()`，BMI088 走独立 `Bmi088StreamDecoder` 分支。
* 当前 `ParserKind` 仅支持 `Auto` / `KeyValue` / `Measurements` / `Bmi088`，其中 `Auto` 只会尝试文本解析。
* 当前 `BusMessage` / `UiBusEvent` 已支持通用文本消息与 BMI088 专用二进制遥测事件，但还没有通用多协议事件层。
* MAVLink 是标准线协议，具备帧头、长度、CRC 和成熟 Rust 生态，适合作为内置自动解析协议。
* Crazyflie CRTP 常见于 radio/USB link，但 Bitcraze 文档也定义了 `CRTP over serial` 的串口分帧格式，可作为本项目的串口内置协议目标。

## Assumptions (temporary)

* 本轮目标优先是串口接入能力，不覆盖 Crazyradio / Crazyflie USB 原生 transport 全栈支持。
* 本轮更偏向“接收、自动识别、结构化展示”，不是完整控制面与参数写入能力。
* 实现会优先复用现有 BMI088 二进制专用分支经验，而不是把二进制协议硬塞进文本 parser。

## Open Questions

* 无阻塞问题，MVP 范围已确认。

## Requirements (evolving)

* 增加 MAVLink 作为自带协议。
* 增加 Crazyflie CRTP 作为自带协议。
* 支持自动识别并自动解析，不要求用户手动逐次切换协议才能看到结构化结果。
* 尽量复用现有 Rust 后端统一消息总线，而不是把协议解析下放到前端。
* 本轮仅支持 `CRTP-over-serial`，不纳入 Crazyradio / USB 原生 transport。
* 本轮优先实现接收、自动识别、结构化总线输出与基础展示，不要求完整控制面与高级交互。
* 当前增量优先补齐 `MAVLink/CRTP` 的协议内容映射与结构化字段，不扩发送侧能力，不新增专门协议面板。
* 当前增量进一步把 `ui/src/store/appStore.ts` 中的 MAVLink/CRTP 展示格式化逻辑抽成独立 helper 模块，降低 store 复杂度。

## Acceptance Criteria (evolving)

* [ ] 串口运行时可以识别并解析 MAVLink 输入流。
* [ ] 串口运行时可以识别并解析 Crazyflie CRTP-over-serial 输入流。
* [ ] 自动模式下不会把随机字节或普通文本高概率误判为 MAVLink 或 CRTP。
* [ ] 新协议解析结果可以进入现有总线并被 Tauri/UI 消费。
* [ ] 本轮不要求 Crazyradio / USB transport 可用，CLI 与行为描述不会误导用户认为已完整支持 Crazyflie 全链路。
* [ ] MAVLink 常见消息至少能输出比 `msg_id/sys/comp` 更有业务意义的结构化字段。
* [ ] CRTP 常见 port/channel 至少能输出比 `port/channel` 更有业务意义的结构化字段。
* [ ] 现有基础展示路径可以直接消费这些增强字段，而不是只显示原始包摘要。
* [ ] `appStore.ts` 不再内联维护大段 MAVLink/CRTP formatter，展示逻辑迁移到独立 helper 且行为保持一致。

## Technical Approach

在现有文本 parser 与 BMI088 二进制专用分支之间，新增可扩展的多协议二进制解码层。`Auto` 模式下并行尝试文本、BMI088、MAVLink、CRTP-over-serial 候选解码器，通过连续成功包与校验通过率锁定协议，避免单包误判。协议解析在 Rust 后端完成，并通过统一消息总线把结构化结果发给 Tauri/UI；前端只消费后端归一化后的事件，不重复做协议解码。

## Decision (ADR-lite)

**Context**: 需要在有限范围内把 Crazyflie CRTP 与 MAVLink 做成自带协议，同时兼顾当前串口架构与后续可扩展性。

**Decision**: 本轮采用务实 MVP，只支持 `MAVLink + CRTP-over-serial` 的自动识别、自动解析、总线接入与基础展示，不覆盖 Crazyradio / USB 原生 transport。

**Consequences**: 范围更可控，能快速复用现有 BMI088 二进制解码经验；但 Crazyflie 的更完整生态接入需要后续单独扩展 transport 层，不能在本轮对外宣称已完整支持。

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* Probe-rs、CAN、TCP/UDP 等新 transport。
* 完整 Crazyradio dongle 工作流实现。
* Crazyflie USB 原生 transport。
* 全量 MAVLink 方言与所有消息类型可视化。

## Technical Notes

* 关键文件：`src/app/mod.rs`、`src/parser.rs`、`src/cli.rs`、`src/message.rs`、`src-tauri/src/model/bus.rs`、`ui/src/store/eventIngestHelpers.ts`。
* 现状说明：当前通用自动识别仅覆盖文本 parser；二进制协议目前只有 BMI088 独立解码链路。
* 初步架构建议：为多协议新增并行候选 decoder + 连续成功阈值锁定的自动识别层。
* 当前继续完善的方向是“协议内容补全”，即补更多 MAVLink 常见消息字段映射和 CRTP 常见 port 语义，不扩发送控制和专门协议页。
* 当前结构整理方向是把 MAVLink/CRTP 的 display formatter 从 `appStore.ts` 抽离，保留现有 ingestion 路径但减轻 store 文件体积与认知负担。
