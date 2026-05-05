# fix self-describing catalog short-string compatibility

## Goal

Make the host accept the device's current command and variable catalog payloads so the self-describing handshake can advance past identity and complete catalog transfer on real hardware.

## What I already know

* The raw self-describing outer transport now works.
* Identity decode now works via a short-string compatibility path.
* The device accepts compact `HostAck(Identity)` and then emits `FrameType = 0x03`, which is `CommandCatalogPage`.
* The observed command catalog payload begins like `03 00 00 01 00 04 03 61 72 6D ...`, which is consistent with:
  * `frame_type = 0x03`
  * `page = 0`
  * `total_pages = 1`
  * `count = 4`
  * first command id length `= 3` -> `arm`
* `decode_command_catalog_page()` and `decode_variable_catalog_page()` currently use only `decode_string()` with `u16` length prefixes.
* The most likely next device behavior after command catalog is a variable catalog page that uses the same short-string convention for names/units.

## Requirements

* Add short-string compatibility to command catalog decoding.
* Add short-string compatibility to variable catalog decoding.
* Keep the compatibility scope limited to catalog payload decoding in this task.
* Preserve canonical host encoding and already-supported decode behavior for other frame types.
* Add focused tests for observed or representative short-string command/variable catalog payloads.

## Acceptance Criteria

* [ ] Command catalog payloads with short string lengths decode successfully.
* [ ] Variable catalog payloads with short string lengths decode successfully.
* [ ] Existing self-describing codec and handshake tests still pass.
* [ ] No unrelated string-decoding paths are broadened in this task.

## Definition of Done

* Backend compatibility change stays limited to catalog decoding.
* Relevant Rust tests pass.

## Technical Approach

Apply the same narrow fallback idea used for identity, but only inside `decode_command_catalog_page()` and `decode_variable_catalog_page()`. Try canonical `u16` strings first; if the parse fails in the same layout-driven way, fall back to `u8` short-string decoding for catalog-local fields only.

## Decision (ADR-lite)

**Context**: The device is now clearly past identity and compact ACK handling, and the next failure occurs inside catalog string decoding.

**Decision**: Add short-string compatibility across both command and variable catalog decoding in one task.

**Consequences**: This increases the chance of completing the whole handshake in one pass while still keeping compatibility scoped to catalog frames rather than all protocol strings.

## Out of Scope

* Changing firmware
* Broad string compatibility for all self-describing frames
* Reworking the wire contract beyond catalog decoding

## Technical Notes

* Target file: `src/protocols/self_describing/codec.rs`
* Command catalog decode: `decode_command_catalog_page()`
* Variable catalog decode: `decode_variable_catalog_page()`
* Existing identity compatibility path is the local model to follow, but this task should not widen beyond catalogs.
