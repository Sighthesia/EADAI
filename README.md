# Embedded Analyzer Design for AI

## Desktop Workbench

- Frontend workspace lives in `ui/`
- Tauri desktop shell lives in `src-tauri/`
- Existing Rust serial runtime remains in `src/`

### Run the frontend

1. Install frontend dependencies: `npm --prefix ui install`
2. Install Tauri CLI if needed: `cargo install tauri-cli --version '^2' --locked`
3. Start the desktop app in development mode: `cargo --manifest-path src-tauri/Cargo.toml tauri dev`

### Why `127.0.0.1:1420` may refuse connection

- `tauri.conf.json` is configured with `devUrl = "http://127.0.0.1:1420"`
- That URL is provided by the Vite dev server from `ui/`
- `cargo run --manifest-path src-tauri/Cargo.toml` does not start the Vite dev server
- `cargo tauri dev -- --manifest-path src-tauri/Cargo.toml` forwards `--manifest-path` to the app runner, which breaks Cargo
- `cargo --manifest-path src-tauri/Cargo.toml tauri dev` starts it correctly through `beforeDevCommand`

### Production build

- Build desktop bundle: `cargo tauri build --manifest-path src-tauri/Cargo.toml`

### Existing CLI

- Tests: `cargo test`
- List ports: `cargo run -- ports`
- Reader loop: `cargo run -- run --port <device> --baud 115200`
