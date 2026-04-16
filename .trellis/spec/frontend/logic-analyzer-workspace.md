# Logic Analyzer Workspace

> Executable reference for the Linux-first sigrok MVP page.

---

## Boundary Overview

The logic analyzer feature is split across a control panel and a view panel:

```text
ui/src/components/ConnectionPanel.tsx
  -> ui/src/store/appStore.ts
  -> ui/src/lib/tauri.ts
  -> src-tauri/src/commands.rs
  -> src-tauri/src/state.rs
  -> src-tauri/src/logic_analyzer.rs
  -> sigrok-cli

ui/src/components/LogicAnalyzerPage.tsx
  -> ui/src/store/appStore.ts
  -> ui/src/lib/tauri.ts
  -> src-tauri/src/commands.rs
  -> src-tauri/src/state.rs
  -> src-tauri/src/logic_analyzer.rs
  -> sigrok-cli
```

Connection control stays in `ConnectionPanel`; waveform viewing stays in `LogicAnalyzerPage`.

---

## Command Contract

Tauri commands exposed by `src-tauri/src/commands.rs`:

| Command | Input | Output |
|---------|-------|--------|
| `get_logic_analyzer_status` | none | `LogicAnalyzerStatus` |
| `refresh_logic_analyzer_devices` | none | `LogicAnalyzerStatus` |
| `start_logic_analyzer_capture` | `LogicAnalyzerCaptureRequest` | `LogicAnalyzerStatus` |
| `stop_logic_analyzer_capture` | none | `LogicAnalyzerStatus` |

### Required request fields

- `LogicAnalyzerCaptureRequest`
  - `deviceRef: string`
  - `sampleCount: number`
  - `samplerateHz?: number | null`
  - `channels: string[]`

### Required status fields

- `LogicAnalyzerStatus`
  - `available`
  - `executable`
  - `sessionState`
  - `devices[]`
  - `selectedDeviceRef`
  - `activeCapture`
  - `lastScanAtMs`
  - `scanOutput`
  - `lastError`
  - `capturePlan`
  - `linuxFirstNote`

---

## Validation Rules

- `deviceRef` must be non-empty before capture starts.
- `sigrok-cli` is resolved from `EADAI_SIGROK_CLI` first, then `PATH`.
- Missing `sigrok-cli` should surface as a non-fatal unavailable state, not a crash.
- Capture start and stop must update the visible page state immediately.

---

## Good / Base / Bad Cases

### Good

- User opens the Connection tab, refreshes devices, selects one, and starts a capture.
- User switches to the Logic Analyzer tab and sees the latest captured waveform with the same full-stage presentation style used by waveform and IMU views.

### Base

- `sigrok-cli` is absent and the page shows the Linux-first help note plus an unavailable state.
- No devices are found after a scan and the page still remains usable.

### Bad

- Duplicating connection controls inside both `ConnectionPanel` and `LogicAnalyzerPage`.
- Adding waveform decode or protocol decoding in this MVP page.
- Hiding sigrok errors instead of surfacing them in `lastError`.

---

## Required Tests

- `tests/logic_analyzer.rs`
  - Parse `sigrok-cli --scan` output into device entries.
  - Build capture commands with optional samplerate and channel lists.
- `npm --prefix ui run build`
  - Verifies the React/Tauri bridge types stay in sync.
- `cargo test --manifest-path src-tauri/Cargo.toml`
  - Verifies the desktop backend wiring remains valid.
