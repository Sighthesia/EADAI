# Workbench Runtime and Scripts Refactor

## Goal

Refactor the `Runtime` and `Scripts` tabs so they stop feeling like dense multi-purpose dashboards and instead guide the user through one primary task at a time.

The target operating model is:

- `Runtime` = observe + act + diagnose
- `Scripts` = browse + edit + runtime impact

## Requirements

### Runtime

- Keep one compact top-level summary area that answers the current runtime state quickly.
- Make command sending the primary workspace block.
- Keep raw serial inspection available, but demote it below the primary action area.
- Move protocol detail, device catalog detail, trigger history, and definition-link explanation behind progressive disclosure instead of giving each equal first-screen weight.
- Remove repeated metadata where the same protocol/device/telemetry context is already visible elsewhere in the same panel.

### Scripts

- Replace the large summary-card deck with a compact authoring overview.
- Keep lane selection and definition browsing together as the browse workspace.
- Keep the editor as the dominant pane.
- Demote runtime context into a compact impact panel instead of a competing summary surface.
- Preserve existing editing behavior for protocol, hook, and variable definitions.

### Shared UX / Visual Rules

- Reuse the current variable-card-inspired visual language from the workbench UI pass.
- Preserve the current dark theme and panel styling direction.
- Prefer structural simplification and progressive disclosure over adding more cards.
- Use `details` or equivalent low-risk disclosure patterns for secondary diagnostics.

### Responsiveness

- Keep the dock-width experience readable at medium and narrow panel widths.
- Avoid layouts where multiple secondary sections compete side-by-side in the first scroll viewport.

## Acceptance Criteria

- `Runtime` has a visible primary action path and no longer presents all diagnostic/reference sections as equal peers.
- `Scripts` has a clearly dominant browse/edit workflow with less top-level summary noise.
- Repeated protocol/device/telemetry context is reduced across both panels.
- Secondary detail is still accessible through disclosure, not removed entirely.
- `npm --prefix ui run build` passes.

## Definition of Done

- Runtime and Scripts structure updated in React components
- Shared styles updated to support the new disclosure/workspace layout
- Frontend build passes

## Technical Approach

- Refactor Runtime from a flat multi-section inspector grid into a staged flow:
  - top summary
  - primary workspace row
  - diagnostics disclosure group
- Refactor Scripts from summary-heavy layout into:
  - compact authoring overview
  - lane browser
  - dominant editor
  - compact runtime impact strip
- Keep store wiring intact; only component composition and presentation should change.

## Decision (ADR-lite)

**Context**: Both tabs currently mix monitoring, operation, reference, and authoring context with duplicated metadata and equal visual weight.

**Decision**: Reorganize by user task flow instead of by raw data categories.

**Consequences**:

- Pros: clearer focus, less duplicated context, better first-screen clarity
- Cons: some deep protocol/catalog data becomes one click further away

## Research References

- [`research/runtime-scripts-information-architecture.md`](research/runtime-scripts-information-architecture.md) — Recommended direction is Runtime = observe + act + diagnose, Scripts = browse + edit + runtime impact.

## Out of Scope

- No store or backend data model changes
- No new persistence behavior for script drafts
- No new protocol/runtime capabilities
- No git commit or PR creation in this task

## Technical Notes

- Current Runtime entrypoint: `ui/src/components/RuntimePanel.tsx`
- Current Scripts entrypoint: `ui/src/components/ScriptsPanel.tsx`
- Runtime detail sections currently live in dedicated subcomponents and can be re-ordered or wrapped without changing store behavior.
