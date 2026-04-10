## 1. 项目概述 (Overview)
本项目是一款开源的、下一代嵌入式综合调试工作站。有别于传统的面向人类开发的串口助手（如 SSCOM、VOFA+），NexusTerm 的核心设计理念是 **“AI 与人协同工作 (Human-AI Teaming)”**。它不仅提供极致性能的数据可视化界面，还内置了标准的 MCP Server，作为大语言模型（Coding Agent）与底层物理硬件交互的“桥梁”和“眼睛”。

### 1.1 核心价值
* **AI 赋能**：允许 AI Agent 直接读取硬件状态、下发指令、甚至获取波形图快照进行多模态视觉调参（如 PID 波形分析）。
* **极致性能**：基于 Rust 底层，支持高频（1000Hz+）传感器数据的零拷贝处理与平滑渲染。
* **无缝扩展**：支持从最简单的纯文本解析，到极简 C 结构体二进制协议，最终演进到基于 `probe-rs` 的零侵入内存直读。

## 2. 系统架构设计 (Architecture)
系统采用 **前后端分离 + 统一消息总线** 的架构。后端完全使用 Rust 构建，确保 I/O 性能和内存安全。

### 2.1 模块分层
1. **Hardware I/O (硬件接入层)**: 
   - 抽象出 `DeviceIO` Trait。
   - MVP 阶段实现基于 `serialport` 的串口驱动；预留 UDP/TCP、CAN 甚至 `probe-rs` (DAP-Link) 接口。
2. **Data Pipeline (数据管道层)**:
   - **Framer (分包器)**: 处理字节流的黏包问题（按换行符或二进制包头/校验和分包）。
   - **Parser (解析器)**: 正则提取、JSON 解析、自定义二进制结构体反序列化。
3. **Unified Message Bus (统一消息总线)**:
   - 采用 Rust `tokio::sync::broadcast` 或无锁环形缓冲区 (Ring Buffer)。
   - 所有的解析后数据（时间戳、通道 ID、数值）在此汇聚。
4. **Consumers (消费者应用层)**:
   - **前端 UI (Tauri IPC/SSE)**: 负责高速波形图 (Echarts/uPlot) 和 3D 姿态模型 (Three.js) 渲染。
   - **MCP Server**: 将总线数据和硬件控制接口暴露给外部 AI 智能体。
   - **Scripting Engine**: 基于 `Rhai` 或 `mlua` 的脚本引擎，执行实时数据过滤与异常触发逻辑。

## 3. MCP 接口规范 (AI 交互核心)
系统将启动一个基于 `stdio` 或 `SSE` 的 MCP Server 服务。

### 3.1 资源 (Resources)
供 Agent 随时读取的只读上下文：
* `serial://{port}/status`：当前串口连接状态、波特率、误码率。
* `device://info`：设备返回的元数据（固件版本、协议字典）。

### 3.2 工具 (Tools)
允许 Agent 调用的执行函数：
* `send_raw_command(cmd: String, format: "ascii"|"hex")`: 发送基础指令。
* `set_parameter(channel: String, value: f64)`: 针对结构化协议的参数下发（如 `set_parameter("pid_p", 1.5)`）。
* `get_channel_statistics(channel: String, window_ms: u32)`: 获取某通道在过去 N 毫秒内的数据统计（最大值、最小值、方差、均值），供 AI 进行纯数值判断。
* **`get_plot_snapshot(channel_id: String)` [🌟核心多模态特性]**: 
  - 触发后端将过去 2 秒的实时波形数据渲染为图片（使用 Rust 后端绘图库如 `plotters` 渲染，或通知前端截图）。
  - 返回 Base64 格式图像内容，供 GPT-4o / Claude 3.5 Sonnet 等多模态模型“视觉分析”超调量和振荡情况。

## 4. 数据解析与处理插件
### 4.1 协议适配策略 (丰俭由人)
* **Level 1: 纯文本正则 (Regex/JSON)**：嵌入式端零修改，使用 `printf`。NexusTerm 使用预设的正则表达式提取数据。
* **Level 2: C-Struct**：提供单头文件 ，以包含包头 `0xAA 0xBB`、负载和 `CRC16` 的形式发送紧凑二进制数据，提升倍解析性能。
* **Level 3: Probe-rs 内存透视 (后期扩展)**：直接通过 SWD 读取 `.elf` 文件对应的内存地址，实现零侵入变量追踪。

### 4.2 热重载数据处理脚本
内嵌 `Rhai` 脚本引擎，允许用户或 **AI 直接下发脚本** 修改数据流转逻辑，例如：
```rust
// AI 通过 MCP 动态注入的预处理脚本，实现快速滤波或单位换算
fn process_imu(raw_val) {
    return raw_val * 9.81 / 16384.0; // 将原始 ADC 值转为 m/s^2
}
```

## 5. 安全与人机协同 (Safety & HIL)
针对 AI 操控硬件可能带来的物理危险，设计如下安全机制：
1. **参数软限幅 (Soft-Limits)**：配置文件中可锁定参数区间（如 `0 < PID_P < 10`），拦截 AI 的越界操作并返回越界错误让 AI 重新思考。
2. **AI 提案模式 (Proposal Mode)**：AI 调用的所有修改命令默认不会直接下发到硬件，而是在前端 UI 弹窗提示：*“AI Agent 建议将 Pitch 轴 P 参数调整为 2.1”*。必须由人类工程师点击 **Approve** 后才执行。
3. **全局急停 (E-Stop)**：提供醒目的软/硬件全局急停快捷键，一键切断下发通道并发送停止报文。

## 6. MVP (最小可行性产品) 实施计划
为了快速验证核心逻辑，第一阶段 (MVP) 的边界如下：

* **后端 (Rust)**：
  - 实现 `serialport` 基本的开启/关闭/读写。
  - 实现基于按行读取的纯文本正则表达式解析器 (提取 `key:value` 格式)。
  - 跑通 MCP Server 协议，实现 `get_statistics` 和 `send_raw_command`。