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

- Current executable truth: this repo is a single Rust CLI crate today, not a Tauri/React app yet. Treat `ROADMAP.md` as target architecture, not current wiring.

## Commands

- Build/test: `cargo test`
- List serial ports: `cargo run -- ports`
- Continuous serial reader: `cargo run -- run --port <device> --baud 115200`
- One-shot send: `cargo run -- send --port <device> --payload "key:value"`
- Loopback verification: `cargo run -- loopback-test --port <device> --payload "ping:1"`
- Interactive terminal: `cargo run -- interactive --port <device> --baud 115200`

## Code Layout

- CLI entrypoint is `src/main.rs`; command parsing lives in `src/cli.rs`.
- Serial I/O lives in `src/serial.rs`; line framing and write/readback helpers are there.
- Runtime supervision and auto-reconnect live in `src/app.rs`.
- Message contract and local bus live in `src/message.rs` and `src/bus.rs`.
- Current parser is only `key:value` in `src/key_value_parser.rs`.

## Working Rules

- Keep backend changes minimal and split by responsibility; do not grow `src/main.rs` into a catch-all.
- Add focused Rust tests for framing, parsing, serial write/readback, and stateful behavior when touching those paths.
- Prefer current CLI/runtime behavior over prose docs if they conflict; several `.trellis/spec/backend/*.md` files still describe an earlier scaffold state.

## Frontend Status

- There is no frontend code, workspace, or web build config in the repo yet.
- If starting frontend work, first create the app scaffold and define the backend-to-UI transport explicitly instead of assuming Tauri IPC already exists.
