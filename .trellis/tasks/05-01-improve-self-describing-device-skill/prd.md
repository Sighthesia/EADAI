# 完善自描述协议设备侧 Skill

## Goal

增强现有 `.agents/skills/self-describing-protocol-integration/SKILL.md`，让它不仅说明协议规则，还能直接指导设备侧/固件侧工程师落地实现。增强内容同时包含最小实现骨架伪代码、关键帧示例、推荐错误码约定，并保持整体精简，不把 skill 膨胀成冗长文档。

## What I already know

* 现有 skill 已覆盖协议阶段、目录结构、位图 sample 编码、SetVariable/AckResult 语义、CRTP 一期承载、常见坑和验证清单。
* 现有 skill 更偏“规则说明 + 接入清单”，还不够像可直接照着写设备侧代码的接入 guide。
* 用户希望增强方向是三项都补：设备侧伪代码/实现骨架、关键帧示例、错误码约定，但要求保持精简。
* 仓库已有相关参照风格：`bmi088-host-handshake-sync` 和 `protocol-handshake-debugging` skill。

## Assumptions (temporary)

* 这次任务只增强 skill 文档和项目 skill 索引，不改协议实现本身。
* 为了控制 token 成本，skill 主体应优先保留高信号内容，细节示例可按需要拆到 reference 文件。
* 设备侧伪代码应保持语言中立，避免过度绑定某一种 MCU SDK 或 RTOS。

## Open Questions

* 当前无阻塞问题；文档结构已确认使用单个 `SKILL.md` 并保持精简。

## Requirements (evolving)

* 增强 `.agents/skills/self-describing-protocol-integration/SKILL.md`。
* 补入最小设备侧实现骨架/伪代码。
* 补入关键协议帧示例。
* 补入推荐错误码约定，帮助设备侧返回一致的 `AckResult.code`。
* 整体保持精简，优先保留高信号、直接指导实现的内容，并继续放在单个 `SKILL.md` 中，不额外拆出 `references/*.md`。
* 更新 `AGENTS.md` 中对应 skill 描述（如有必要），确保触发语义与增强后的 skill 保持一致。

## Acceptance Criteria (evolving)

* [ ] skill 能指导设备侧工程师从上电到 streaming 完成最小协议接入。
* [ ] skill 给出最小可执行的设备侧状态机/伪代码骨架。
* [ ] skill 包含关键帧示例和推荐错误码约定。
* [ ] skill 内容仍保持精简，没有变成冗长总文档。

## Definition of Done (team quality bar)

* Skill content updated and internally consistent
* AGENTS skill index updated if needed
* No redundant files created unless they clearly improve progressive disclosure
* Wording is actionable and trigger-friendly

## Out of Scope (explicit)

* 修改协议实现代码
* 修改 Tauri/UI 展示
* 增加与当前协议无关的新 skill

## Technical Notes

* 主要修改目标：`.agents/skills/self-describing-protocol-integration/SKILL.md`、`AGENTS.md`
* 可参考风格：`.agents/skills/bmi088-host-handshake-sync/SKILL.md`、`.agents/skills/protocol-handshake-debugging/SKILL.md`
