# support raw self-describing framing in host

## Goal

Make the host accept the device's actual self-describing wire format (`0x73 + len + payload`, no outer CRC) so the existing self-describing session and staged HostAck flow can finally run on real hardware instead of stalling before frame decode.

## What I already know

* The device is emitting `0x73 0x17 ...` frames that match the project skill and backend spec for raw self-describing transport.
* The current host `CrtpDecoder` only accepts `CRTP-over-serial` framing: `[header][len][payload][crc8]`.
* The self-describing session logic is implemented after CRTP packet decode and therefore never runs if the outer framing fails.
* The backend spec already states that raw self-describing transport is `0x73 + len + payload` with no outer CRC.
* Current CRTP/self-describing tests are built around the CRC-bearing CRTP-over-serial wrapper, which does not match the skill's executable transport contract.

## Assumptions (temporary)

* The host should be brought into alignment with the raw self-describing transport contract instead of changing the device firmware back toward the older CRTP-over-serial wrapper.
* Existing non-self-describing CRTP-over-serial support should remain available.

## Requirements

* Detect and decode raw self-describing outer frames only in the `auto` path: `header=0x73`, `len`, `payload`, no outer CRC.
* Feed decoded payloads into the existing self-describing codec/session path.
* Reuse the existing self-describing staged `HostAck` flow for responses.
* Preserve existing CRTP-over-serial support for normal CRTP traffic and keep explicit `crtp` parser semantics unchanged in this task.
* Avoid false CRTP console/channel-0 empty-packet matches on raw self-describing identity streams.

## Acceptance Criteria

* [ ] Raw self-describing identity frames decode successfully on the host in `auto` mode.
* [ ] The host can emit staged `HostAck` replies through the same raw transport in `auto` mode.
* [ ] Existing CRTP-over-serial tests still pass or are intentionally updated with explicit scope.
* [ ] Focused tests cover raw framing decode and handshake progression.
* [ ] Explicit `crtp` parser behavior remains unchanged.

## Definition of Done (team quality bar)

* Tests added or updated for framing and handshake behavior
* Lint / typecheck / CI green for touched layers
* Docs/notes updated if behavior changes

## Technical Approach

Add a lightweight raw self-describing frame detector for the `auto` parser path that recognizes the canonical `0x73 + len + payload` transport described by the backend spec and device skill. When such a frame is found, decode the payload with the existing self-describing codec/session and send response frames back using the same raw transport. Keep the current CRC-bearing `CrtpDecoder` and explicit `crtp` parser path unchanged.

## Decision (ADR-lite)

**Context**: The real device currently emits raw self-describing frames, while the host only accepts CRC-bearing CRTP-over-serial before entering the self-describing session.

**Decision**: Implement raw self-describing support only in `auto` as the smallest change that unblocks real hardware without redefining explicit `crtp` parser semantics.

**Consequences**: Real-device auto-connect can succeed without forcing firmware changes, while explicit `crtp` remains a separate, older transport path. A later task may still add a dedicated explicit parser for raw self-describing if needed.

## Out of Scope (explicit)

* Device-side firmware rewrites
* Replacing all CRTP support with raw self-describing transport
* UI redesign beyond any strictly necessary parser exposure changes
* Redefining explicit `crtp` parser mode in this task

## Technical Notes

* Current auto path lives in `src/app/mod.rs`
* Current CRTP framing lives in `src/protocols/crtp.rs`
* Self-describing framing/codec lives in `src/protocols/self_describing/`
* Relevant spec source of truth: `.trellis/spec/backend/serial-multi-protocol.md`
* Relevant device-side contract: `.agents/skills/self-describing-protocol-integration/SKILL.md`
