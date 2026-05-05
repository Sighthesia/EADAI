# add protocol parser selection and fix mismatched startup handshake

## Goal

Prevent the runtime from starting the wrong protocol handshake on links that actually speak self-describing CRTP, and add an explicit protocol parser selection path so the user can choose the intended parser instead of relying on the current BMI088 fallback.

## What I already know

* `src-tauri/src/state.rs` maps missing or unknown `request.parser` to `ParserKind::Bmi088`.
* `src-tauri/src/model/session.rs::ConnectRequest` already supports an optional `parser` field on the Rust side.
* `ui/src/types/session.ts::ConnectRequest` does not currently expose a `parser` field.
* `ui/src/store/appStore.ts` config state also does not currently store a parser selection.
* `ui/src/components/ConnectionPanel.tsx` has serial/fake connection controls but no protocol parser selector.
* `src/app/mod.rs` only sends BMI088 boot commands and schema retries when `parser == ParserKind::Bmi088`.
* The observed failure is a protocol mismatch: host keeps sending BMI088 `REQ_SCHEMA` while the device is sending self-describing `0x73 + len + Identity` frames.

## Assumptions (temporary)

* The intended MVP is to let the user choose the parser explicitly and avoid accidental BMI088 startup on self-describing links.
* The existing auto-detect path should stay available.

## Requirements

* Add end-to-end parser selection support from UI store -> Tauri `ConnectRequest` -> runtime config.
* Expose parser choices in the connection UI with the current built-in values: `auto`, `bmi088`, `mavlink`, `crtp`, `key_value`, `measurements`.
* Change the default serial parser from `bmi088` to `auto` so multi-protocol links do not start with a BMI088-only handshake unless explicitly requested.
* Ensure the selected parser controls whether BMI088 boot/retry commands are emitted.
* Keep self-describing/CRTP links from being forced through BMI088 startup behavior when the user selects a non-BMI088 parser.
* Preserve fake-stream and existing serial connection flows aside from the parser-default change.

## Acceptance Criteria

* [ ] The connection UI exposes protocol parser selection without adding excessive chrome.
* [ ] The chosen parser reaches the backend runtime config.
* [ ] Missing parser now defaults to `auto` instead of `bmi088` on the desktop connection path.
* [ ] Non-BMI088 selections do not emit BMI088 startup retries.
* [ ] Relevant tests cover parser propagation or startup gating.

## Definition of Done (team quality bar)

* Tests added/updated where appropriate
* Lint / typecheck / CI green for touched layers
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Technical Approach

Extend the frontend session config shape to carry a parser selection, render that choice in the existing compact connection details panel, and forward it through the Tauri `ConnectRequest` into `runtime_config()`. At the backend boundary, change the fallback parser from `ParserKind::Bmi088` to `ParserKind::Auto`, while preserving the existing BMI088 boot/retry gating so those commands only fire when the selected parser is explicitly `bmi088`.

## Decision (ADR-lite)

**Context**: The runtime currently defaults serial sessions to `bmi088`, which is no longer a safe assumption now that the same UI/runtime path supports CRTP and self-describing protocol traffic.

**Decision**: Add explicit parser selection and change the default serial parser to `auto`.

**Consequences**: This reduces accidental protocol mismatch on mixed-protocol links and makes parser choice visible to the user, but it changes the historical default behavior for users who implicitly relied on BMI088-only startup.

## Out of Scope (explicit)

* Rewriting the entire protocol auto-detect architecture
* Device-side firmware changes
* Adding brand-new protocol decoders beyond the existing parser enum

## Technical Notes

* Backend parser default currently lives in `src-tauri/src/state.rs`
* Rust-side request shape lives in `src-tauri/src/model/session.rs`
* Frontend connection config lives in `ui/src/types/session.ts`, `ui/src/store/appStore.ts`, and `ui/src/components/ConnectionPanel.tsx`
* Runtime handshake gating lives in `src/app/mod.rs`
* Frontend panel should stay compact per `.trellis/spec/frontend/workbench-visual-language.md`
