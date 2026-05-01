<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

## Subagents

- ALWAYS wait for all subagents to complete before yielding.
- Spawn subagents automatically when:
  - Parallelizable work (e.g., install + verify, npm test + typecheck, multiple tasks from plan)
  - Long-running or blocking tasks where a worker can run independently.
  - Isolation for risky changes or checks

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->

# EADAI Agent Notes

- Executable truth: this repo is a Rust serial library/CLI in `src/` plus a Tauri shell in `src-tauri/` and a React/Vite UI in `ui/`. Treat `ROADMAP.md` as target architecture, not exact runtime wiring.

## High-Value Commands

- Root verification: `cargo test`
- Tauri verification: `cargo test --manifest-path src-tauri/Cargo.toml`
- Frontend verification: `npm --prefix ui run build`
- Desktop dev: `cargo --manifest-path src-tauri/Cargo.toml tauri dev`
- Do not use `cargo run --manifest-path src-tauri/Cargo.toml` for desktop dev; it does not start the Vite server from `beforeDevCommand`.
- CLI entrypoints: `cargo run -- ports`, `cargo run -- run --port <device> --baud 115200`, `cargo run -- send --port <device> --payload "key:value"`, `cargo run -- interactive --port <device> --baud 115200`

## Structure That Matters

- `src/main.rs` is only the CLI shell; keep runtime logic out of it.
- `src/app.rs` supervises serial runtime + reconnect flow.
- `src/message.rs` and `src/bus.rs` define the cross-layer event contract; `Analysis`/`Trigger` events now flow through this path.
- `src/parser.rs`, `src/key_value_parser.rs`, and `src/measurement_parser.rs` normalize incoming serial text.
- `src/analysis/` is the time-domain analysis layer; prefer extending it over recomputing metrics in Tauri/UI.
- `src-tauri/src/` adapts the Rust bus into UI-facing events; `ui/src/store/appStore.ts` is the main frontend ingestion point.

## Repo-Specific Rules

- Preserve the fake-stream desktop debug path unless the task explicitly changes debugging ergonomics; the workbench expects it for local verification.
- When changing serial, parser, analysis, or reconnect behavior, add focused Rust tests under `tests/`; this repo keeps tests out of source files.
- There is no dedicated frontend lint script today; the real frontend gate is `npm --prefix ui run build` (`tsc --noEmit` + Vite build).
- Prefer executable config over Trellis prose when they disagree; some `.trellis/spec/backend/*.md` files still describe earlier scaffolding.

## Skills

| Skill                                           | When to use                                                                                  |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `.agents/skills/tauri-streaming-debug/SKILL.md` | Tauri fake stream, waveform rendering, cursor/overlay sync, store batching, or high-frequency UI update debugging |
| `.agents/skills/bmi088-host-handshake-sync/SKILL.md` | BMI088 UART4 schema-first handshake, host parser, ACK/START flow, or shared debug/telemetry port debugging |
| `.agents/skills/protocol-handshake-debugging/SKILL.md` | Generic schema-first binary protocol, mixed text/binary stream, handshake, CRC, or high-rate telemetry debugging |
| `.agents/skills/self-describing-protocol-integration/SKILL.md` | Firmware-side adoption of the generic self-describing device protocol: identity, catalog handshake, bitmap-compressed samples, variable write-back |
