# 完整接入 Crazyflie CRTP 与 MAVLink 协议

## Goal

为 EADAI 完整接入 Crazyflie CRTP 与 MAVLink 两套协议能力，包括可扩展 transport、字节流分帧、协议语义映射、统一能力总线与后续控制扩展边界；本任务不把“串口收包展示 MVP”误当成完整协议支持，而是以完整协议架构为目标，分阶段落地。

## What I already know

* 当前运行时主链路在 `src/app/mod.rs`：串口读字节后，文本走 `parser::parse_framed_line()`，BMI088 走独立 `Bmi088StreamDecoder` 分支。
* 当前仓库已经存在 `MavlinkDecoder` 与 `CrtpDecoder`，并已接入 Rust bus、Tauri model、UI 展示链路。
* 当前 `ParserKind` 已暴露 `mavlink` 与 `crtp`，`Auto` 也会尝试 BMI088 / MAVLink / CRTP / 文本。
* 当前 Crazyflie 支持仍然只是 `CRTP-over-serial` 接收与展示，不包含 Crazyradio dongle 或 USB native transport。
* 当前 MAVLink 支持仍然主要是帧解析、部分常见消息字段映射与展示，不等于完整 MAVLink dialect / command / mission / param 能力支持。
* 现有实现已经证明“解码并接入展示链路”可行，但尚未形成面向协议演进的 transport / framing / semantic / capability 分层。

## Assumptions (temporary)

* 本任务允许分阶段交付，但目标定义必须是“完整协议接入架构”，不是继续停留在 serial-only MVP。
* 一期可以先落地最关键骨架与 serial transport，但架构上必须预留 Crazyradio / USB native / MAVLink dialect 演进位。
* 实现会优先复用现有 BMI088 和当前 MAVLink/CRTP 增量成果，但需要把现有逻辑整理为可扩协议层，而不是继续堆特判。

## Open Questions

* 当前无阻塞问题，一期范围已确认包含 Crazyradio transport 落地。

## Requirements (evolving)

* 把协议支持拆分为 `transport / framing / semantic / capability` 四层，避免把协议名直接等同于某个串口 parser。
* 完整接入 MAVLink：至少覆盖可演进的 dialect/version 策略、收包解析、统一消息/能力输出，以及后续发送/ACK/command 扩展边界。
* 完整接入 Crazyflie：至少覆盖 CRTP 语义层、serial transport、Crazyradio transport 落地、USB native 的可接入边界，以及后续 commander/log/param 能力扩展边界。
* 一期交付除 serial transport 外，还要真实落地 Crazyradio transport；接口设计不能阻塞后续 USB native 接入。
* 支持自动识别并自动解析，不要求用户手动逐次切换协议才能看到结构化结果。
* 尽量复用现有 Rust 后端统一消息总线，而不是把协议解析下放到前端。
* UI 优先消费统一能力事件；协议原始包事件保留用于调试与协议专属展示。
* 现有 `appStore.ts` 中的 MAVLink/CRTP 展示逻辑继续保持抽离方向，避免协议细节继续侵入主 store。

## Acceptance Criteria (evolving)

* [ ] 运行时协议接入点从当前“串口 parser 特判”升级为可扩展的 `transport / framing / semantic / capability` 分层。
* [ ] 串口运行时可以识别并解析 MAVLink 输入流。
* [ ] 串口运行时可以识别并解析 Crazyflie CRTP-over-serial 输入流。
* [ ] 自动模式下不会把随机字节或普通文本高概率误判为 MAVLink 或 CRTP。
* [ ] MAVLink 与 CRTP 的原始协议包事件可以进入现有总线并被 Tauri/UI 消费。
* [ ] 至少有一组统一能力事件能够承接跨协议共性语义，避免 UI 未来继续按协议重复造字段展示逻辑。
* [ ] Crazyradio transport 在本项目里具备真实可运行的接入路径，而不是只停留在接口预留或文档声明。
* [ ] CLI、文档与行为描述不会把当前阶段能力误导为“已完整支持所有 Crazyflie transport / 全量 MAVLink 能力”。
* [ ] MAVLink 与 CRTP 的语义映射采用集中可演进策略，而不是把协议版本/消息语义长期散落在多个 switch 中。
* [ ] 一期设计明确 Crazyradio / USB native / MAVLink dialect 升级的后续接入位与不破坏兼容的演进方式。

## Technical Approach

以分层方式重构当前运行时：`transport` 只负责连接与收发字节流，`framing` 负责从字节流切出协议帧，`semantic` 负责从帧提炼结构化语义，`capability` 负责沉淀跨协议的业务能力事件。当前已存在的 serial + MAVLink/CRTP 逻辑作为一期落地点接入该骨架；一期同时真实落地 Crazyradio transport，使 Crazyflie 不再只是 `CRTP-over-serial` 的接收展示。USB native 与更完整 MAVLink dialect/version 演进通过同层扩展完成，而不是继续把逻辑塞回 `ParserKind` 和 UI store。

## Decision (ADR-lite)

**Context**: 需要把当前已经存在的 MAVLink/CRTP 接收展示 MVP 升级为真正面向长期演进的完整协议接入方案，同时避免对外误称“已完整支持”但内部仍是串口特判与手写映射堆叠。

**Decision**: 本任务以完整协议接入架构为目标，采用分阶段实现：一期先把现有 serial + MAVLink/CRTP 逻辑收敛到可扩展分层骨架，并同时真实落地 Crazyradio transport；后续阶段再按 USB native 与更完整 capability 扩展逐步补齐。

**Consequences**: 一期实现复杂度会高于单纯补 parser 与 formatter，但能显著降低后续每新增 transport、消息类型或协议版本时的重构成本，也能避免 UI 与运行时继续被协议细节侵蚀。

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* 与 Crazyflie / MAVLink 无关的新协议接入。
* 一次性补齐所有 MAVLink dialect、全部消息类型和全部地面站能力。
* 一次性补齐 Crazyflie 全部 app-layer 工作流与完整飞控控制面。
* Crazyflie USB native transport 的一期内真实落地。

## Technical Notes

* 关键文件：`src/app/mod.rs`、`src/cli.rs`、`src/message.rs`、`src/protocols/`、`src-tauri/src/model/bus.rs`、`ui/src/store/appStore.ts`、`ui/src/store/protocolDisplayHelpers.ts`。
* 现状说明：当前已经存在基础 MAVLink/CRTP 解码与展示链路，但主要是 serial-only + 协议专属事件与语义映射。
* 初步架构建议：新增协议 runtime 骨架，显式分离 transport、framing、semantic、capability 四层，并让 auto-detect 只作用于 framing/runtime 层。
* 一期工作重点不再只是“补更多字段”，而是先把协议接入方式从功能堆叠改成可演进架构。
* 当前已知风险：如果继续沿用手写消息语义 + UI formatter 扩张，后续支持 Crazyradio/USB/MAVLink 版本演进时维护成本会快速上升。
* 新增 Crazyradio 意味着需要核实现有 Rust 生态、宿主平台依赖和测试可行性；如果底层库能力不足，需要在一期内至少把 transport 适配层和降级行为设计清楚。
