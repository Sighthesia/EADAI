# fix self-describing staged host ack

## Goal

Fix the host-side self-describing handshake so it acknowledges each required stage in order, allowing device-side command and variable catalogs to flow after identity instead of stalling at repeated identity frames.

## What I already know

* The host decodes the incoming self-describing identity frame correctly over CRTP debug channel 3.
* `src/app/mod.rs` already forwards decoded self-describing frames into `SelfDescribingSession::on_frame(...)`.
* The current host session only auto-sends `HostAck(VariableCatalog)` and `HostAck(Streaming)`.
* The device-side protocol skill requires staged acknowledgments in this order: `Identity -> CommandCatalog -> VariableCatalog`.
* The current symptom is repeated identity packets with no command or variable catalog frames following.

## Requirements

* When the host receives a valid `Frame::Identity` and advances to `WaitingCommandCatalog`, it must emit `HostAck { stage: Identity }` exactly once for that stage transition.
* When the host finishes receiving the full command catalog and advances to `WaitingVariableCatalog`, it must emit `HostAck { stage: CommandCatalog }` exactly once for that stage transition.
* The existing variable catalog acknowledgment behavior must remain intact.
* Add clearer session-stage logs so stalled handshakes can be diagnosed from host logs without reading raw bytes only.
* Do not change CRTP port/channel mapping, frame codec, or device-side framing assumptions in this task.

## Acceptance Criteria

* [ ] A session test proves `Frame::Identity` returns one `HostAck(Identity)` response and moves the handshake to `WaitingCommandCatalog`.
* [ ] A session test proves the final `CommandCatalogPage` returns one `HostAck(CommandCatalog)` response and moves the handshake to `WaitingVariableCatalog`.
* [ ] A session test proves `VariableCatalogPage` still returns `HostAck(VariableCatalog)` and existing stream activation behavior is preserved.
* [ ] Logs clearly show the handshake stage progression at identity, command catalog, and variable catalog boundaries.
* [ ] `cargo test` passes for the touched protocol/session coverage.

## Definition of Done

* Tests added or updated for the staged self-describing handshake behavior.
* Rust code remains small and explicit, with no new unnecessary abstraction layers.
* Verification covers protocol regression risk on the self-describing host path.

## Technical Approach

Update `src/protocols/self_describing/session.rs` so `SelfDescribingSession::on_frame()` emits staged `HostAck` responses immediately after successful state transitions for identity and command catalog completion, matching the device-side executable contract. Extend focused session tests to pin the full staged handshake behavior and add concise diagnostic logs around stage completion.

## Decision (ADR-lite)

**Context**: The host and device already agree on frame encoding, transport, and channel mapping, but the host session does not acknowledge the early handshake stages required by the device-side contract.

**Decision**: Fix the host session logic instead of relaxing the device or adding compatibility shims. Add minimal stage logs while keeping the protocol contract unchanged.

**Consequences**: The host behavior will align with the canonical staged handshake and should unblock catalog transfer immediately. Existing self-describing tests must be updated because they currently encode the wrong handshake expectation.

## Out of Scope

* Changing device-side firmware behavior or introducing fallback protocol dialects.
* Changing CRTP framing, self-describing codec layout, or app-level bus event shapes.
* Adding new UI behavior beyond what existing backend events already project.

## Technical Notes

* Relevant runtime path: `src/app/mod.rs`
* Relevant host session path: `src/protocols/self_describing/session.rs`
* Relevant state machine path: `src/protocols/self_describing/state.rs`
* Relevant protocol contract: `.trellis/spec/backend/serial-multi-protocol.md`
* Relevant BMI088 host contract cross-check: `.trellis/spec/backend/bmi088-host-protocol.md`
* Device-side source of truth for staged acknowledgments: `.agents/skills/self-describing-protocol-integration/SKILL.md`
