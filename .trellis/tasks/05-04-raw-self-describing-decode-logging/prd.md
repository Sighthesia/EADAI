# log raw self-describing decode failures

## Goal

Add concise diagnostics for raw self-describing payload decode failures so host logs can reveal the exact inner protocol mismatch instead of silently falling through to misleading CRTP false positives.

## What I already know

* Raw outer framing is now recognized in `ParserKind::Auto`.
* The current `RawSelfDescribingDecoder` silently drops payload decode errors by draining one byte and continuing.
* Current user logs still show repeated `0x73 0x17 ...` plus fake `CRTP console/channel=0/payload_len=0` packets, which strongly suggests `decode_frame(payload)` is failing.
* The logging spec allows concise runtime parse failure logs as long as they avoid unbounded binary dumps.

## Requirements

* In the raw self-describing decoder, log decode failures from `decode_frame(payload)`.
* The log must include enough information to distinguish frame-layout drift from random noise:
  * decode error kind
  * payload length
  * bounded hex preview
* Keep the logging bounded and compact.
* Do not change protocol behavior in this task beyond diagnostics.

## Acceptance Criteria

* [ ] Raw self-describing decode failures produce a visible diagnostic line.
* [ ] The diagnostic includes error type, payload length, and bounded preview.
* [ ] Existing protocol tests still pass.

## Definition of Done

* Focused backend change only
* No protocol semantics changed
* Relevant tests pass

## Out of Scope

* Fixing the inner payload layout mismatch itself
* Changing host/device framing contracts further

## Technical Notes

* Target file: `src/protocols/self_describing/crtp_adapter.rs`
* Logging constraints: `.trellis/spec/backend/logging-guidelines.md`
