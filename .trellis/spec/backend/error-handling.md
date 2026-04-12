# Error Handling

> How errors are handled in this project.

---

## Overview

The current backend has no custom error framework yet.
`src/main.rs` is still the default template entry point, so there is no established error type hierarchy, client response format, or propagation pattern.
When real backend logic is added, prefer a single project-wide error type and keep conversion points close to the boundary that handles the failure.

---

## Error Types

No custom error types are defined yet.
If the backend starts handling I/O, parsing, or protocol failures, define project-specific error variants for those domains instead of using raw strings.

---

## Error Handling Patterns

No standard pattern exists yet.
For future Rust backend code, propagate recoverable errors with `Result` and reserve panics for programmer bugs or impossible states.

---

## API Error Responses

The MCP boundary in `src/mcp_server.rs` now uses rmcp `ErrorData` responses.
Documented MCP cases live in `.trellis/spec/backend/mcp-ai-contract.md` and should remain the source of truth for:

- `resource_not_found` on unknown resource URIs
- `invalid_params` on unknown tools or invalid tool arguments
- `internal_error` on serialization failures

---

## Common Mistakes

Avoid:

- Returning ad-hoc error strings from multiple layers
- Hiding failures behind `unwrap()` or `expect()` in runtime code
- Logging the same error repeatedly at every layer
