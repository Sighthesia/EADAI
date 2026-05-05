# fix self-describing catalog page decode mode selection

## Goal

Make command and variable catalog pages decode using one consistent string-length mode per page so real-device catalog payloads stop drifting mid-page and failing on bogus value types.

## What I already know

* Identity compatibility is already working.
* Compact raw HostAck is already working.
* Catalog short-string compatibility was added, but the real device still fails on variable catalog pages.
* The observed variable catalog payload starts with a valid first record (`acc_x`, `order=0`, `unit=raw`, `value_type=I16`) and then later fails with `invalid value type: 108` or `11`.
* This pattern strongly suggests the decoder is switching string modes per field instead of per page. In particular, empty strings or short units can be mis-read as valid canonical zero-length strings, causing the cursor to drift by one byte and corrupt the following `value_type`.

## Requirements

* Decode command catalog pages using a single chosen string mode for the whole page.
* Decode variable catalog pages using a single chosen string mode for the whole page.
* The decoder may try canonical mode first and short-string mode second, but once a page mode is chosen it must be used consistently across all strings in that page.
* Apply the per-page mode strategy to both command and variable catalog pages in this task.
* Keep compatibility scoped to catalog pages only.
* Add focused tests for multi-entry pages where mixed field shapes (including empty strings) would fail under per-field fallback.

## Acceptance Criteria

* [ ] Command catalog page decoding uses a stable per-page mode.
* [ ] Variable catalog page decoding uses a stable per-page mode.
* [ ] Focused regression tests cover a page that would fail under per-field fallback.
* [ ] Existing protocol tests still pass.

## Definition of Done

* Backend-only fix
* Relevant Rust tests pass

## Technical Approach

Refactor catalog page decoders so they parse an entire page in one mode at a time instead of calling a per-field fallback helper. Attempt canonical `u16` strings first; if the page parse fails in a layout-driven way, retry the entire page with `u8` short strings.

## Decision (ADR-lite)

**Context**: Per-field fallback appears too permissive because empty canonical strings can look valid and shift the cursor for later fields.

**Decision**: Choose decode mode once per page.

**Consequences**: Catalog compatibility becomes more robust without widening string fallback beyond catalogs.

## Out of Scope

* Changing firmware
* Broad string compatibility for all self-describing frames

## Technical Notes

* Target file: `src/protocols/self_describing/codec.rs`
* Current problematic helper: `decode_catalog_string()`
* The fix should likely replace per-field fallback with per-page retry.
