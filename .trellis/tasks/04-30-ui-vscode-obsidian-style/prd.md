# Modernize UI With VS Code-Obsidian Hybrid Style

## Goal

Refresh the desktop UI so it feels like a modern flat workbench inspired by VS Code structure and Obsidian texture, while preserving the current task-first docked workflow and avoiding business-logic changes.

## What I already know

* The frontend is a Vite + React app in `ui/src/`.
* The shell and status chrome live in `ui/src/App.tsx`.
* The docked workspace is powered by `flexlayout-react` in `ui/src/components/Workbench.tsx`.
* Shared visual styling is concentrated in `ui/src/styles.css`.
* Existing UI already uses a dark theme but leans toward gradient-heavy industrial panels.

## Assumptions (temporary)

* This task should primarily update presentation, not workflows or information architecture.
* The desired direction is a hybrid: VS Code-style shell and hierarchy with Obsidian-like softness.

## Open Questions

* None blocking for the first pass.

## Requirements (evolving)

* Replace the current heavier glossy styling with a flatter dark workbench language.
* Make the main shell, side docks, tabs, status strip, and shared controls feel closer to a modern desktop IDE.
* Preserve current layout behavior, panel structure, and existing functionality.
* Apply changes through shared style primitives where possible.

## Acceptance Criteria (evolving)

* [ ] The app shell reads visually as a flatter IDE-style workspace.
* [ ] Flexlayout tabs, docks, and borders look coherent with the new theme.
* [ ] Shared buttons, inputs, cards, and disclosure surfaces match the new visual language.
* [ ] Frontend build succeeds.

## Definition of Done (team quality bar)

* Tests added/updated where appropriate
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* Reworking panel information architecture
* Replacing `flexlayout-react`
* Adding a user-configurable theming system

## Technical Notes

* `ui/src/App.tsx`
* `ui/src/components/Workbench.tsx`
* `ui/src/styles.css`
* `.trellis/spec/frontend/workbench-visual-language.md`
