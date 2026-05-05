# align self-describing protocol skill example

## Goal

Align the self-describing protocol skill documentation and portable device reference example with the host's current canonical wire contract so future firmware integrations do not repeat the string-length drift we just debugged.

## What I already know

* The current portable reference emits raw outer framing correctly: `0x73 + len + payload`.
* The current portable reference already emits compact two-byte `HostAck` payloads.
* The current portable reference still encodes `Identity`, command catalog, and variable catalog strings with one-byte length prefixes via `sdp_put_string()`.
* The host's canonical encoder still uses `u16` string lengths for `Identity`, command catalog, and variable catalog payloads; host-side short-string support was added only as a compatibility fallback for existing device drift.
* The skill text currently does not explicitly call out the string length width for identity/catalog string fields, which makes the example drift easy to reintroduce.

## Requirements

* Update `.agents/skills/self-describing-protocol-integration/SKILL.md` so the executable contract explicitly states that identity/catalog strings use canonical host framing and `u16 LE` string lengths.
* Update the portable reference under `.agents/skills/self-describing-protocol-integration/reference/` so it emits canonical `u16` string lengths for identity and catalog fields.
* Preserve the compact two-byte `HostAck` device contract and raw outer transport.
* Keep the example application-layer and portable; do not add host-compat fallback logic to the device reference.

## Acceptance Criteria

* [ ] Skill documentation explicitly describes canonical string-length encoding for identity/catalog fields.
* [ ] Portable reference no longer emits one-byte string lengths for identity/catalog fields.
* [ ] `HostAck` remains exactly `0x04 + stage`.
* [ ] Reference docs remain internally consistent with the skill.

## Definition of Done

* AI-context docs and skill files stay concise and action-oriented.
* Reference example matches the documented wire contract.

## Technical Approach

Replace the reference example's short-string helper with a canonical `u16` string encoder, update the identity and catalog builders to use it, and clarify the contract in `SKILL.md` and `reference/README.md` so the portable example remains the authoritative device-side pattern for future firmware work.

## Decision (ADR-lite)

**Context**: The repo's host had to grow compatibility code because the example/reference encoded strings more compactly than the canonical host framing expected.

**Decision**: Keep host compatibility for existing devices, but tighten the skill and reference example back to the canonical `u16` string contract for future firmware implementations.

**Consequences**: Future device integrations should line up with the documented host contract immediately, while existing legacy devices may still rely on host compatibility shims until firmware is updated.

## Out of Scope

* Removing host compatibility support for already-deployed drifted devices
* Reworking sample framing or bitmap semantics

## Technical Notes

* Skill doc: `.agents/skills/self-describing-protocol-integration/SKILL.md`
* Portable reference: `.agents/skills/self-describing-protocol-integration/reference/self_describing_device_portable.c`
* Reference overview: `.agents/skills/self-describing-protocol-integration/reference/README.md`
