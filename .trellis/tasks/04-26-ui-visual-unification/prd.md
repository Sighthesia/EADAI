# Workbench Runtime and Scripts Refactor

## Goal

Refactor the `Runtime` and `Scripts` tabs so they stop feeling like dense multi-purpose dashboards and instead guide the user through one primary task at a time.

User refinement on 2026-04-27: the `Runtime` surface should now become a `Terminal` surface focused on serial RX/TX history plus hook and script execution visibility, while preserving the broader visual-unification goals for the right dock.

The target operating model is:

- `Runtime` = observe + act + diagnose
- `Scripts` = browse + edit + runtime impact

## Requirements

### Runtime

- Keep one compact top-level summary area that answers the current runtime state quickly.
- Rename the `Runtime` panel to `Terminal` in the dock UI and in-panel copy.
- Make terminal history the primary workspace block instead of a multi-card runtime inspector.
- Split the main terminal workspace vertically:
  - upper half = received information
  - lower half = sent information and send actions
- Keep only those two visible terminal regions; remove top summaries and other visible terminal sections from the main tab surface.
- Use the receive area to show serial receive history plus visible hook / script execution-related activity already available from runtime events.
- Use the send area to show serial send history and a send composer.
- Replace the current command-center emphasis with a collapsible, scrollable command list sourced from device/runtime/script-derived command metadata already available in the app.
- Clicking a command with no parameters should send it immediately.
- Clicking a command with parameters should populate the input box with the command template and focus the parameter portion for immediate editing.
- Add an explicit terminal payload encoding display switch that allows toggling between `binary`, `hex`, and `ASCII` views.
- In the receive area, detect consecutive packets whose structure stays the same and only numeric values change.
- For those consecutive same-shape packets, avoid repeating the full pasted/raw block for every row.
- Instead, align and emphasize the changing numeric segments with red/green background rectangles that indicate value changes.
- Show a repeat/change count badge in the packet row corner for these condensed consecutive updates.
- Keep raw/protocol/catalog diagnostics available only through progressive disclosure instead of giving each equal first-screen weight.
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

- `Terminal` replaces `Runtime` as the right-dock operational panel name and primary interaction surface.
- `Terminal` has a clear two-pane RX/TX workspace and no longer presents all diagnostic/reference sections as equal peers.
- `Terminal` shows only the receive pane and send pane in the main tab surface.
- The lower send pane includes a collapsible, scrollable command list that supports one-click send or one-click command templating with parameter focus.
- `Terminal` lets the user switch payload display between `binary`, `hex`, and `ASCII`.
- Repeated receive packets with stable structure and only numeric changes are visually condensed, with numeric deltas highlighted and a repeat/change count visible.
- `Scripts` has a clearly dominant browse/edit workflow with less top-level summary noise.
- Repeated protocol/device/telemetry context is reduced across both panels.
- Secondary detail is still accessible through disclosure, not removed entirely.
- `npm --prefix ui run build` passes.

## Definition of Done

- Terminal and Scripts structure updated in React components
- Shared styles updated to support the new disclosure/workspace layout
- Frontend build passes

## Technical Approach

- Refactor Runtime into Terminal and move from a flat multi-section inspector grid into a staged flow:
  - top summary
  - primary terminal workspace
  - diagnostics disclosure group
- Drive the terminal body from existing console, protocol, trigger, hook, and runtime command data where possible; prefer frontend composition/state changes over backend contract changes.
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
