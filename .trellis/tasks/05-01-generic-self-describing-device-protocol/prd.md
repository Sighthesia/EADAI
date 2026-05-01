# 通用自描述设备协议

## Goal

为 EADAI 设计并实现一套与底层 transport 解耦的通用自描述设备协议，让设备可以在会话启动时向主机声明身份、命令目录、变量目录与握手状态，在握手完成后按变量定义顺序持续发送二进制 telemetry 数据，并允许主机通过同一二进制协议修改可调变量值。一期要求把“数值未变压缩标记”也纳入协议设计与实现，而不是只做全量 sample 流。

## What I already know

* 当前仓库已经有分层协议骨架：`transport / framing / semantic / capability`。
* 当前仓库已有 BMI088 专用 host protocol，已支持 identity/schema/sample/ack/start 与部分调参命令，但它不是通用自描述协议。
* 当前仓库的 CRTP 支持可作为协议第一种底层承载链路，但协议模型本身不能写死在 CRTP 上。
* 用户希望的协议流程包含：设备信息 -> 主机应答 -> 命令定义 -> telemetry 变量定义 -> 主机逐项确认 -> 握手完成 -> 流式 sample -> 主机变量写回。
* 用户已确认：新协议按“通用协议”设计，不是 CRTP 专用；一期还要包含“数值未变压缩标记”。

## Assumptions (temporary)

* 一期协议优先接入 CRTP 链路，但模型与状态机保持 transport-agnostic。
* 一期可以先不做变量顺序动态重排，只要变量目录里有顺序定义并能稳定驱动 sample 解码。
* 命令目录与变量目录优先服务运行时/UI 自动发现，不要求一开始就做完整富文本文档渲染。

## Open Questions

* 当前无阻塞问题；需求边界已收敛，可以进入实现准备。

## Requirements (evolving)

* 定义一套与 transport 解耦的通用自描述设备协议。
* 协议至少包含以下语义帧：`Identity`、`CommandCatalog`、`VariableCatalog`、`HostAck`、`TelemetrySample`、`SetVariable`。
* 握手状态机至少覆盖：设备身份上报、命令目录同步、变量目录同步、主机确认、开始 streaming。
* 主机确认采用“每个阶段一个总 ACK”模式，而不是每个目录项逐条 ACK。
* 变量目录至少包含：变量名、顺序、单位、是否可调。
* 变量值编码采用目录声明的定宽数值类型（如 `u16/i16/u32/f32`），不在一期内支持可变长值类型。
* 命令目录至少包含：命令标识、参数描述、基础文档文本或说明字段。
* 命令目录与变量目录允许分页/批量分段传输，并通过阶段完成标记结束该目录同步。
* `TelemetrySample` 除全量数值外，还必须支持“数值未变压缩标记”，并采用位图批量标记未变化字段。
* 主机可以通过二进制协议写入被标记为可调的变量值，且写值后必须由设备回执确认成功/失败，再让主机侧状态生效。
* 变量写回、命令执行结果与阶段确认统一复用 `Ack/Result` 回执帧，而不是为命令单独定义响应帧。
* 一期优先落在 CRTP 链路上，但不要把状态机和数据模型写死在 CRTP port/channel 语义里。
* 协议解析、握手和变量更新逻辑尽量放在 Rust 后端，不把二进制协议解释工作下放到前端。

## Acceptance Criteria (evolving)

* [ ] 仓库内存在一套独立于 transport 的通用自描述协议数据模型。
* [ ] 设备身份、命令目录、变量目录、主机确认和 sample streaming 有明确的协议帧与状态机定义。
* [ ] 一期实现支持“数值未变压缩标记”的编码/解码逻辑。
* [ ] 主机可以对可调变量执行二进制写回。
* [ ] CRTP 可作为该协议的一种可运行底层承载方式。
* [ ] 前端或 Tauri 层不需要重新解码二进制 sample 或目录协议。

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* 一期内同时落地所有 transport（如 USB native、serial、MAVLink 承载）
* 一期内做完整变量顺序动态重排
* 一期内做完整富文档命令帮助系统

## Technical Notes

* 相关现有能力：BMI088 host protocol、CRTP transport/runtime、Rust bus/Tauri projection。
* 该任务与 `05-01-crazyflie-crtp-mavlink` 有关联，但目标层级更高，应作为独立协议任务推进。
