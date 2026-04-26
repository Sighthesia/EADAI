# Workbench Runtime Panel Integration Analysis

## Goal
Analyze and define a UI optimization plan for the serial workbench so operators can understand the active protocol, handshake/runtime state, hook configuration, and recent hook activity without jumping between disconnected panels.

## Requirements
- Analyze the current split between `Serial Console`, `Protocol`, and `Script Hooks` in the workbench layout.
- Propose a more integrated information architecture that makes runtime state visible at a glance.
- Preserve the existing shared store model where possible instead of introducing duplicate state.
- Make the relationship between raw serial traffic, protocol parsing/handshake, and hook execution explicit in the UI.
- Include recommendations for what should be shown as persistent summary state versus drill-down detail.

## Acceptance Criteria
- [ ] The task documents the current panel split and the main operator visibility gaps.
- [ ] The task proposes at least one concrete integrated layout or panel-composition direction.
- [ ] The task defines how the UI should expose active protocol, handshake phase, hook availability, and recent hook trigger/runtime activity in one glance.
- [ ] The task identifies the main frontend files and shared state sources involved in the redesign.
- [ ] The task keeps protocol parsing behavior out of scope unless presentation changes reveal a minimal missing UI-facing field.

## Technical Approach
Use the existing frontend workbench layout and Zustand store as the source of truth. Focus first on presentation, state synthesis, and operator workflow clarity rather than changing backend protocol behavior.

The likely design direction is a unified runtime status surface: a compact always-visible summary for protocol + hooks + latest traffic, with deeper drill-down sections for raw console, handshake timeline, schema details, and hook examples/history.

## Decision (ADR-lite)
**Context**: The right-side workbench runtime information is currently split across three tabs, making it hard to correlate protocol state, raw traffic, and hook execution.

**Decision**: Create a dedicated planning task focused on frontend information architecture and runtime visibility before changing protocol abstraction or handshake implementation.

**Consequences**: This keeps scope small and actionable, reduces UI confusion first, and allows later protocol abstraction work to build on a clearer operator-facing surface.

## Out of Scope
- Refactoring the BMI088 protocol implementation itself.
- Changing handshake semantics or parser logic unless a tiny UI-facing state gap is identified.
- Implementing a generic script engine redesign.
- Reworking unrelated panels such as waveforms, MCP, or logic analyzer.

## Technical Notes
- `ui/src/components/Workbench.tsx` currently places `Serial Console`, `Protocol`, and `Script Hooks` as separate right-side tabs.
- `ui/src/components/ConsolePanel.tsx` already exposes raw traffic and BMI088 frame diagnostics.
- `ui/src/components/ProtocolPanel.tsx` already exposes handshake phase, schema, timeline, and command actions.
- `ui/src/components/ScriptHookPanel.tsx` is mostly static today and does not reflect runtime hook activity.
- `ui/src/store/appStore.ts` already centralizes protocol snapshot state and related runtime ingestion, so the biggest gap appears to be UI composition rather than backend data capture.
- Related types live in `ui/src/types.ts`, including `UiProtocolSnapshot`, `UiProtocolHandshakeEvent`, and `UiTriggerPayload`.
