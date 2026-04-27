# Frontend Development Guidelines

> Best practices for frontend development in this project.

---

## Overview

The UI is a Vite + React app in `ui/src/`.
The current pattern is a small app shell plus focused workspace components.

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Logic Analyzer Workspace](./logic-analyzer-workspace.md) | Independent logic-analyzer page and Tauri bridge contract | Filled |
| [Variable Card Tuning Contract](./variable-card-tuning-contract.md) | Inline tuning contract for eligible existing variable cards | Filled |
| [Workbench Visual Language](./workbench-visual-language.md) | Compact task-first chrome, progressive disclosure, and summary-strip conventions | Filled |

---

## Current Frontend Structure

- `ui/src/App.tsx` hosts a unified workbench shell.
- `ui/src/components/Workbench.tsx` owns the dockable panel layout.
- `ui/src/components/ConnectionPanel.tsx` owns both serial connection controls and logic-analyzer capture controls.
- `ui/src/components/RuntimePanel.tsx` owns the integrated protocol/traffic/hook summary surface.
- `ui/src/components/LogicAnalyzerPage.tsx` owns the logic-analyzer visualization tab.
- `ui/src/lib/tauri.ts` holds Tauri command wrappers.
- `ui/src/types.ts` mirrors Rust payloads for the desktop bridge.

---

**Language**: All documentation should be written in **English**.
