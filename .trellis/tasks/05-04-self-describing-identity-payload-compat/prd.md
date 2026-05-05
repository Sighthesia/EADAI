# fix self-describing identity payload compatibility

## Goal

Make the host accept the device's current self-describing identity payload layout so the raw transport path can advance past `Identity` instead of failing immediately with `truncated data`.

## What I already know

* Raw self-describing outer framing now reaches the host and logs decode failures.
* The observed payload is `01 01 06 43 59 54 34 42 42 03 30 2E 31 64 00 00 00 1E 00 04 00 3C 00`.
* `decode_identity()` currently calls `decode_string()` twice, and `encode_string()` / `decode_string()` use a `u16` length prefix on the host side.
* The device payload clearly uses `u8` string lengths for `device_name` and `firmware_version`:
  * `06` -> `CYT4BB`
  * `03` -> `0.1`
* After those two strings, the remaining fixed-width numeric fields line up with the skill contract:
  * `sample_rate_hz = 100`
  * `variable_count = 30`
  * `command_count = 4`
  * `sample_payload_len = 60`

## Assumptions (temporary)

* The current firmware likely uses the same short-string convention beyond identity, not just for this one frame.
* The desired outcome is to keep host compatibility localized and explicit instead of silently weakening all decode rules.

## Requirements

* Decode the current device identity payload successfully.
* Limit the compatibility change to `Identity` only in this task.
* Preserve existing canonical host encode/decode behavior for already-supported frames.
* Add focused tests for the observed device identity payload shape.
* After successful decode, the existing handshake path should be able to advance to `HostAck(Identity)` so the next logs can reveal whether catalogs need a follow-up task.

## Acceptance Criteria

* [ ] The observed identity payload decodes into the expected `Identity` fields.
* [ ] Existing self-describing codec tests still pass.
* [ ] New compatibility logic is covered by focused regression tests.
* [ ] Compatibility is explicitly scoped to identity payload decoding.

## Definition of Done

* Backend compatibility change stays small and explicit.
* Relevant Rust tests pass.

## Technical Approach

Teach `decode_identity()` to accept the current device's short-string layout for identity only, while keeping the canonical host encoder unchanged. The compatibility path should be narrow and deterministic, driven by the observed payload structure instead of weakening all string decoding globally.

## Decision (ADR-lite)

**Context**: The device payload uses one-byte string lengths for identity, while the host codec currently expects two-byte string lengths everywhere.

**Decision**: Fix only identity compatibility first.

**Consequences**: This is the fastest path to progress the real handshake and observe the next failure point. If command or variable catalogs use the same short-string convention, a follow-up task may still be needed.

## Out of Scope

* Changing the device firmware
* Reworking the entire self-describing codec format

## Technical Notes

* Host codec: `src/protocols/self_describing/codec.rs`
* Current payload error source: `decode_identity()` after `decode_string()`
* Relevant device-side contract: `.agents/skills/self-describing-protocol-integration/SKILL.md`
