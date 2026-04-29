# mcp page collapsible sections

## Goal

Improve the MCP page by collapsing the current MCP server summary into a compact disclosure, and add a second collapsible area that shows all MCP tools as variable-style cards, including each tool's name and the last time it was called after the app started.

## What I already know

* The current MCP page is `ui/src/components/McpPanel.tsx`.
* The page already shows MCP server status, endpoint, runtime source, and session details in a flat layout.
* The frontend already uses `details`/`summary` disclosure patterns elsewhere in the UI.
* The app store has MCP status, but no existing tool-invocation history or last-called timestamp field.
* The desktop bridge exposes `get_mcp_server_status`, but not per-tool usage metadata.

## Assumptions (temporary)

* Tool call timestamps will be tracked from the moment the desktop app starts.
* If a tool has never been called in the current app session, the UI will show a clear fallback such as `Never`.
* The MCP tool list can be derived from the existing server/tool contract without adding a new backend API unless necessary.

## Open Questions

* Should the "last called time" be tracked only for the current app session, or persisted across restarts?

## Requirements (evolving)

* Wrap the MCP server status block in a collapsible disclosure.
* Add a second collapsible disclosure below it for MCP tools.
* Render each tool as a compact card with tool name and last-called time.
* Keep the page consistent with existing runtime disclosure styling.

## Acceptance Criteria (evolving)

* [ ] The MCP server information is hidden behind a collapsible section.
* [ ] MCP tools are displayed in a second collapsible section below the server info.
* [ ] Each tool card shows a tool name and a last-called timestamp or a fallback when unavailable.
* [ ] The layout still fits the existing workbench visual language.

## Definition of Done

* Frontend changes implemented and verified.
* Any required state or bridge updates added.
* `npm --prefix ui run build` passes.

## Technical Approach

Use the existing disclosure patterns already present in the frontend as the visual and interaction model. Extend the MCP/UI state shape only as far as needed to surface tool metadata and last-call timestamps, while keeping the MCP panel itself focused on summary information.

## Out of Scope

* Reworking the MCP server protocol itself unless needed for tool metadata exposure.
* Persisting tool-call history across app restarts unless explicitly requested.

## Technical Notes

* UI target: `ui/src/components/McpPanel.tsx`
* Shared UI patterns: `ui/src/components/RuntimeProtocolSection.tsx`, `ui/src/components/RuntimeCatalogSection.tsx`
* Store/type surface: `ui/src/store/appStore.ts`, `ui/src/types.ts`
* Tauri bridge: `ui/src/lib/tauri.ts`, `src-tauri/src/commands.rs`, `src-tauri/src/state.rs`
