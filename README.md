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

### MCP server

- Desktop app mode: the Tauri shell now starts one shared read-only MCP server on `http://127.0.0.1:8765/mcp`.
- Shared mode means the desktop UI and MCP clients read the same runtime session, including fake profiles.
- Standalone binary still exists for headless debugging: `eadai-mcp`
- Server binary: `eadai-mcp`
- Build it once: `cargo build --bin eadai-mcp`
- Run against the fake telemetry stream: `cargo run --bin eadai-mcp -- --fake-profile telemetry-lab`
- Run against a real serial device: `cargo run --bin eadai-mcp -- --port <device> --baud 115200`
- Desktop shared mode uses Streamable HTTP at `http://127.0.0.1:8765/mcp`.
- Standalone `eadai-mcp` still uses stdio, so point clients at the built binary when possible. That avoids Cargo's startup noise and keeps the protocol stream clean.

### Claude Desktop config

Claude Desktop reads `claude_desktop_config.json` and launches the MCP server as a local process.

macOS:

`~/Library/Application Support/Claude/claude_desktop_config.json`

Example:

```json
{
  "mcpServers": {
    "eadai-fake": {
      "command": "/home/you/projects/EADAI/target/debug/eadai-mcp",
      "args": ["--fake-profile", "telemetry-lab"]
    },
    "eadai-serial": {
      "command": "/home/you/projects/EADAI/target/debug/eadai-mcp",
      "args": ["--port", "/dev/ttyUSB0", "--baud", "115200"]
    }
  }
}
```

After editing the file, fully quit and reopen Claude Desktop.

### Codex config

Codex reads MCP servers from `~/.codex/config.toml` or a project-scoped `.codex/config.toml`.

Example:

```toml
[mcp_servers.eadai-fake]
command = "target/debug/eadai-mcp"
args = ["--fake-profile", "telemetry-lab"]
cwd = "/home/you/projects/EADAI"
enabled = true

[mcp_servers.eadai-serial]
command = "target/debug/eadai-mcp"
args = ["--port", "/dev/ttyUSB0", "--baud", "115200"]
cwd = "/home/you/projects/EADAI"
enabled = true
```

After editing the file, start a new Codex session or reload MCP settings.
