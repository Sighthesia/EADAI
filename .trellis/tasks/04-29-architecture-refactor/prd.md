# Architecture Refactor

## Goal

Refactor the highest-complexity source files without changing runtime behavior so the repo keeps the current Rust core -> Tauri bridge -> React UI architecture, but large files are split along clearer domain boundaries and become easier to extend safely.

## What I already know

* The repo is structured as a Rust serial library/CLI in `src/`, a Tauri desktop bridge in `src-tauri/`, and a React/Vite UI in `ui/`.
* `src/main.rs` is already a thin CLI shell and `src-tauri/src/main.rs` is already a thin desktop entrypoint.
* The main architecture pressure points are oversized files, not a fundamentally broken layering model.
* The highest-value refactor targets are currently `src/bmi088.rs`, `ui/src/store/appStore.ts`, and `ui/src/components/WaveformPanel.tsx`.
* `src/app.rs`, `src-tauri/src/model.rs`, and `ui/src/types.ts` are the next tier of oversized bridge/runtime files after the first refactor wave.

## Requirements

* Preserve current behavior and public contracts unless a small compatibility adjustment is required internally.
* Split `src/bmi088.rs` into a directory module organized by protocol concerns.
* Split `ui/src/store/appStore.ts` by frontend state domain while keeping one coherent app store API for existing consumers.
* Split `ui/src/components/WaveformPanel.tsx` by rendering/model/overlay concerns while preserving the existing panel behavior.
* Continue with a second refactor wave for `src/app.rs`, `src-tauri/src/model.rs`, and `ui/src/types.ts` using the same behavior-preserving approach.
* Keep changes incremental and reviewable; avoid broad architecture rewrites.
* Run relevant verification after refactoring.

## Acceptance Criteria

* [ ] `src/bmi088.rs` is replaced by smaller focused module files with the same runtime behavior.
* [ ] `ui/src/store/appStore.ts` is reduced substantially by moving logic into focused store modules/slices without breaking callers.
* [ ] `ui/src/components/WaveformPanel.tsx` is reduced substantially by moving model/render helper logic into focused modules without changing the visible UX.
* [ ] `src/app.rs` is reduced substantially by moving runtime loop/support logic into focused backend modules without changing runtime behavior.
* [ ] `src-tauri/src/model.rs` and `ui/src/types.ts` are reduced substantially by splitting bridge models by domain while preserving current contracts.
* [ ] Rust tests pass for touched backend code.
* [ ] Frontend build passes for touched UI/store code.

## Definition of Done

* Tests added or updated where behavior-sensitive refactors need protection.
* `cargo test` passes, or any failing subset is explained if unrelated.
* `npm --prefix ui run build` passes.
* No intentional runtime behavior regression is introduced.

## Technical Approach

Refactor by domain seams instead of by arbitrary line-count splitting:

* Backend protocol domain: split BMI088 constants, models, codec/decoder, and session state.
* Frontend state domain: split store implementation into session/protocol/variables/IMU/logic-analyzer/event-ingest helpers while keeping a stable top-level store export.
* Frontend waveform domain: split panel container, plot lifecycle, plot-model building, and overlay/stat helpers.
* Backend runtime domain: split runtime command flow, publish helpers, and serial loop helpers out of `src/app.rs` while preserving `App` as the external runtime facade.
* Bridge type domain: split Tauri/UI DTOs by session/protocol/logic-analyzer/runtime concerns while keeping stable imports for existing callers.

## Decision (ADR-lite)

**Context**: Several core files have grown past the point where one engineer can safely reason about them quickly.

**Decision**: Perform a behavior-preserving modular refactor focused on the three biggest complexity hotspots first instead of rewriting architecture or introducing new frameworks.

**Consequences**: The codebase gains clearer ownership boundaries and lower merge-conflict pressure, at the cost of more files and a short-term increase in import/export plumbing.

## Out of Scope

* Rewriting the message-bus architecture.
* Replacing Zustand, Tauri, or the Rust runtime model.
* Major UI redesign.
* Re-architecting the transport/message flow beyond modular extraction.

## Technical Notes

* File size hotspots observed during inspection:
* `ui/src/store/appStore.ts` ~1930 LOC
* `ui/src/components/WaveformPanel.tsx` ~1839 LOC
* `src/bmi088.rs` ~1679 LOC
* `ui/src/components/VariablesPanel.tsx` ~1140 LOC
* `src/app.rs` ~569 LOC
* `src-tauri/src/model.rs` ~633 LOC
* `ui/src/types.ts` ~530 LOC
* Relevant specs:
* `.trellis/spec/backend/bmi088-host-protocol.md`
* `.trellis/spec/backend/mcp-ai-contract.md`
* `.trellis/spec/frontend/index.md`
* `.trellis/spec/frontend/logic-analyzer-workspace.md`
