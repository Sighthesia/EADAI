<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

Use the `/trellis:start` command when starting a new session to:
- Initialize your developer identity
- Understand current project context
- Read relevant guidelines

Use `@/.trellis/` to learn:
- Development workflow (`workflow.md`)
- Project structure guidelines (`spec/`)
- Developer workspace (`workspace/`)

Keep this managed block so 'trellis update' can refresh the instructions.

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
| `.agents/skills/tauri-streaming-debug/SKILL.md` | Tauri fake stream, waveform rendering, store batching, or high-frequency UI update debugging |
| `.agents/skills/bmi088-host-handshake-sync/SKILL.md` | BMI088 UART4 schema-first handshake, host parser, ACK/START flow, or shared debug/telemetry port debugging |
| `.agents/skills/protocol-handshake-debugging/SKILL.md` | Generic schema-first binary protocol, mixed text/binary stream, handshake, CRC, or high-rate telemetry debugging |
