# fix raw self-describing compact host ack

## Goal

Make host acknowledgments on the raw self-describing transport match the device's compact two-byte ACK contract so the handshake can advance beyond repeated identity frames.

## What I already know

* The host now successfully decodes raw self-describing identity frames and enters `WaitingCommandCatalog`.
* The host currently logs `sending raw frame payload_len=7` after identity, which means it is sending the canonical codec `HostAck` payload rather than the device's compact ACK form.
* The device-side skill requires raw HostAck payloads to be exactly two bytes:
  * `0x04`
  * `stage`
* After the host sends its current 7-byte ACK, the device repeats identity, which strongly indicates the ACK was rejected.

## Requirements

* On the raw self-describing transport, send compact HostAck payloads that match the device-side contract.
* Unify the host's HostAck representation around the compact two-byte payload instead of keeping a second canonical payload form.
* Preserve the staged handshake flow already implemented in the host session.
* Add focused tests that prove raw ACK encoding is exactly two bytes after the outer transport header and that the staged identity ACK uses the compact form.

## Acceptance Criteria

* [ ] Raw identity-stage HostAck is emitted in compact two-byte form.
* [ ] Existing self-describing session state progression remains unchanged.
* [ ] Existing tests are updated to the compact HostAck contract with explicit scope.
* [ ] Focused raw transport tests cover compact ACK emission.

## Definition of Done

* Small backend-only change
* Relevant Rust tests pass

## Technical Approach

Change the self-describing `HostAck` codec to the compact two-byte device contract (`0x04 + stage`) and align the raw send path plus tests to that single representation. This removes the split between codec-level HostAck payloads and raw transport HostAck payloads.

## Decision (ADR-lite)

**Context**: The current host has two incompatible ideas of HostAck: a longer codec payload with `stage + status + message`, and the device-side raw contract of exactly two bytes.

**Decision**: Unify on the compact two-byte HostAck representation.

**Consequences**: The protocol surface becomes simpler and matches the real device contract, but existing HostAck tests need updating because the prior codec shape is no longer valid.

## Out of Scope

* Redefining the general self-describing codec format
* Firmware changes

## Technical Notes

* Raw send path: `src/app/mod.rs::send_raw_self_describing_frame`
* Canonical codec HostAck: `src/protocols/self_describing/codec.rs`
* Device-side ACK contract: `.agents/skills/self-describing-protocol-integration/SKILL.md`
