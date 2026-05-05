# fix serial write timeout for self-describing handshake

## Goal

Decouple serial write timeout behavior from the short read polling timeout so self-describing handshake ACKs can be written reliably on real hardware without regressing the rest of the serial entrypoints.

## What I already know

* The current runtime can decode identity, command catalog, variable catalog, and generate the correct compact raw `HostAck` frames.
* The real device still repeats identity when the host logs `failed to send raw response: transport error: write failed: Operation timed out`.
* `serialport::new(...).timeout(...)` is currently configured from the same `read_timeout` value used for read polling.
* The default runtime read timeout is short (`50ms`), which is good for polling reads but too aggressive for some writes on real serial drivers.
* The affected code paths include `src/serial.rs` and `src/protocols/serial_transport.rs`.

## Requirements

* Separate serial write-timeout expectations from short read polling timeouts.
* Apply the improved timeout strategy consistently across runtime and other serial open paths, not only one narrow callsite.
* Keep existing read polling behavior responsive.
* Add focused verification for the timeout plumbing or serial open configuration where practical.

## Acceptance Criteria

* [ ] Runtime serial writes no longer depend on the short read polling timeout.
* [ ] Other serial open paths are aligned to the same timeout model or explicitly justified if not.
* [ ] Existing Rust tests still pass.
* [ ] The change remains transport-focused and does not alter protocol semantics.

## Definition of Done

* Backend-only change
* Relevant Rust tests pass

## Technical Approach

Introduce an explicit serial write-timeout policy separate from read polling, wire it through the serial open/wrapper layer, and keep the runtime using short non-blocking-ish reads while allowing writes enough time to complete on real hardware.

## Decision (ADR-lite)

**Context**: The protocol stack now reaches the correct ACK write point, but transport write timeouts abort the handshake before the device can receive the response.

**Decision**: Fix the serial transport timeout model comprehensively rather than only bumping one callsite.

**Consequences**: Serial runtime behavior becomes more robust on real hardware, while preserving the current fast polling read loop.

## Out of Scope

* Protocol framing changes
* Device firmware changes

## Technical Notes

* Runtime transport wrapper: `src/protocols/serial_transport.rs`
* Serial open helpers: `src/serial.rs`
* Runtime config source: `src/cli.rs`
