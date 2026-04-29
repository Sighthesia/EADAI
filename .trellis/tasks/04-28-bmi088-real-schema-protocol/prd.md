# align bmi088 real schema protocol

## Goal

Align the host BMI088 UART4 schema decoder with the device frames that are actually on the wire so handshake can complete, variables can be created in the UI, and the runtime stops looping on `REQ_SCHEMA`.

## What I already know

* The wire envelope is valid: `A5 5A | version | type | cmd | seq | len | payload | crc16`.
* The host can send `REQ_SCHEMA` and the device responds with `EVENT + SCHEMA (0x80)`.
* CRC is validated before schema parsing, so a schema payload that reaches `decode_schema_payload_with_seq()` is a real frame, not random noise.
* The current device schema payload does not match the documented modern `schema_version/rate_hz/field_count/sample_len/...` layout.
* Runtime logs show a legacy-looking payload beginning with `00 0E 00 70 69 64 5F 79 61 77 5F 69 5F 67 61 69 6E 1B 01 00 0E 00 ...`.
* The first decoded field is consistently `pid_yaw_i_gain`, then later bytes look like `field_id + field_type + scale_q + name_len + unit_len` for following fields.
* Handshake loops because `SCHEMA` never decodes successfully, so session phase stays `AwaitingSchema` and auto-retries `REQ_SCHEMA`.

## Assumptions (temporary)

* The current device emits a mixed legacy schema descriptor layout: first field uses a short descriptor, later fields use a 5-byte descriptor header.
* The host should preserve compatibility with both documented modern schema frames and this legacy on-device variant.

## Open Questions

* Whether the mixed legacy layout applies to all fields after the first one or only to a subset of tuning-related fields.

## Requirements (evolving)

* Decode the documented framed schema payload format.
* Decode the currently observed legacy schema payload variant from the device logs.
* Keep schema-derived sample decoding driven by the active schema order.
* Preserve existing host command and event envelope behavior.
* Keep diagnostics clear enough to tell which schema variant was decoded.

## Acceptance Criteria (evolving)

* [ ] Host accepts modern framed schema payloads without regression.
* [ ] Host accepts the observed legacy/mixed schema payload shape and builds fields in order.
* [ ] Session can leave `AwaitingSchema` once a valid device schema arrives.
* [ ] Focused Rust tests cover both schema variants.

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* Redesigning the entire UART4 firmware protocol
* Removing temporary protocol diagnostics in the same change
* Reworking frontend variables rendering unrelated to schema decode

## Technical Notes

* Main runtime decoder: `src/bmi088.rs`
* Diagnostic decoder mirrors runtime parser behavior: `src/bmi088_diag/protocol.rs`
* Contract doc currently overstates the modern 30-field schema as universal truth: `.trellis/spec/backend/bmi088-host-protocol.md`
* Recent runtime evidence shows one valid schema payload with first field `pid_yaw_i_gain` and subsequent bytes shaped like `1B 01 00 0E 00 ...`
