# fix self-describing streaming frame envelope drift

## Goal

Resolve the remaining self-describing streaming mismatch now that handshake succeeds, by converging host/runtime behavior and the portable device reference around the exact telemetry sample envelope actually required on the wire.

## What I already know

* Raw self-describing handshake succeeds through `Identity`, `CommandCatalog`, and `VariableCatalog`.
* Host-side logging now distinguishes pre-handshake, post-handshake, and streaming decode failures, and can hint when a payload looks like a bare sample body missing frame type `0x05`.
* The canonical host codec expects telemetry sample payloads to begin with frame type `0x05`, followed by `seq (u32 LE)`, `bitmap_len (u16 LE)`, `bitmap`, and changed values.
* The latest runtime logs still show post-handshake raw frames like `73 83 00 ...` and `73 61 00 ...`, which decode as `invalid frame type: 0` with the diagnostic hint `likely bare telemetry sample payload missing frame type 0x05`.
* The observed sample frames appear to be exactly one byte shorter than the canonical envelope would be if `0x05` were present, which strongly suggests the active streaming TX path is still emitting the sample body without the outer self-describing sample frame type.
* The portable reference under `.agents/skills/self-describing-protocol-integration/reference/` now explicitly emits `0x05` for telemetry samples and logs sample sends through the optional debug hook.

## Assumptions (temporary)

* The remaining issue is in the active device streaming path or in a second sample TX helper that is still bypassing the canonical frame builder.
* Host-side state progression and raw/CRTP demux are no longer the primary source of the observed failure.
* This task will stay strict on the host side and will not add compatibility decoding for bare sample bodies.

## Open Questions

* None at the moment.

## Requirements (evolving)

* Preserve the canonical self-describing sample contract in docs, reference code, and host codec: raw payload must start with `0x05`.
* Reduce ambiguity in logs so post-handshake sample envelope drift is immediately identifiable from runtime output.
* Do not add host-side compatibility decoding for bare sample bodies in this task.
* Add an explicit runtime verdict when the host has enough post-handshake evidence that the active stream is using a non-canonical sample envelope.
* Surface that verdict as a structured session / bus event so CLI, Tauri, and future UI layers can consume the same protocol-mismatch conclusion.
* Trigger the verdict only after 2-3 consecutive post-handshake hits of the same non-canonical sample-envelope evidence, so half-packets or transient misalignment do not immediately trip the session verdict.
* The structured verdict should contain a fixed reason code plus a compact evidence summary, including the phase, consecutive hit count, first payload byte, payload length, and a short hint.

## Acceptance Criteria (evolving)

* [ ] Host remains strict and does not decode bare sample bodies as valid telemetry samples.
* [ ] Runtime behavior or diagnostics make the remaining sample envelope drift unambiguous.
* [ ] Canonical sample contract remains explicit in the skill/reference materials.
* [ ] The runtime surfaces an explicit verdict for non-canonical streaming frame envelope drift.
* [ ] The verdict payload includes a reason code and compact evidence summary without turning into an unbounded debug dump.

## Decision (ADR-lite)

**Context**: Live logs now strongly indicate the active streaming path is emitting a bare sample body without the canonical `0x05` telemetry frame type, while handshake and catalog frames already follow the raw self-describing contract.

**Decision**: Keep the host strict and do not add compatibility decoding for bare sample bodies in this task. Continue converging runtime diagnostics, tests, and the portable reference toward the canonical sample envelope, and surface the resulting mismatch as a structured session / bus verdict.

**Consequences**: The host stays aligned with the documented wire contract and avoids silently normalizing protocol drift, but real devices still using the non-canonical sample path will continue to fail until their streaming TX path is brought into line with the reference.

## Definition of Done

* Tests added or updated where runtime behavior changes.
* `cargo check` and relevant Rust tests pass.
* Reference/docs stay aligned with the canonical host wire contract.

## Out of Scope (explicit)

* Broad compatibility for arbitrary non-canonical self-describing payloads.
* Reworking handshake staging that is already functioning.
* UI changes unrelated to protocol ingest and diagnostics.

## Technical Notes

* Canonical host sample frame type is defined in `src/protocols/self_describing/codec.rs` as `FRAME_TYPE_TELEMETRY_SAMPLE = 0x05`.
* Raw transport is encoded as `0x73 + len + payload`.
* Current portable reference sample send helper is `sdp_send_sample_frame()` in `.agents/skills/self-describing-protocol-integration/reference/self_describing_device_portable.c`.
* Current raw failure classification lives in `src/protocols/self_describing/crtp_adapter.rs`.
