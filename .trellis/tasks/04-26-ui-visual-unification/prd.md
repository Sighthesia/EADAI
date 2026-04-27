# Workbench UI Visual Unification

## Purpose

Polish the current workbench UI so that the dock tabs, Runtime panel, and Scripts panel feel like one coherent product surface.
The new styling should follow a hybrid direction:
- base card stability from the Variables panel variable cards
- elevated/floating visual quality from the Waveform Controls floating panel

## Primary Targets

1. Dock tab labels and dock button presentation in the workbench layout
2. Runtime panel layout, card hierarchy, spacing, and visual consistency
3. Scripts panel layout, lane switcher, browser/editor presentation, and spacing
4. Cross-panel visual consistency issues in borders, radii, backgrounds, chips, and text hierarchy

## Design Direction

Use a balanced visual language:
- main information cards should feel stable, dense, and readable like variable cards
- controls, tab affordances, and highlighted surfaces may use a lighter floating/glass treatment inspired by the waveform floating panel
- avoid making Runtime or Scripts feel overly decorative or detached from the rest of the workbench

## Requirements

### 1. Tab presentation
- Improve the visual quality of the workbench tab/dock labels
- Make selected, idle, and hover states clearer and more consistent with the rest of the UI
- Keep the tab treatment compatible with the existing flexlayout workbench structure

### 2. Runtime surface cleanup
- Reduce visual noise in Runtime without hiding useful information
- Normalize card hierarchy, spacing, padding, border strength, and headline rhythm
- Fix layout issues caused by rigid summary/inspector grids on medium widths
- Make Runtime visually align with Variables and stage-adjacent surfaces

### 3. Scripts surface cleanup
- Make the lane switcher feel like a deliberate navigation control instead of generic chips
- Improve the browser/editor split so it reads as a unified authoring workspace
- Normalize list item states, selected states, and editor shell styling
- Fix spacing/alignment issues that make the panel feel uneven

### 4. Shared visual system
- Unify repeated values where practical: panel background treatment, border colors, radii, shadows, and compact chip styling
- Preserve existing dark theme and current information density
- Prefer low-risk CSS and small markup refinements over broad component rewrites

### 5. Responsiveness and regression safety
- Add or refine breakpoints for Runtime and Scripts so they remain readable at narrower dock widths
- Preserve the Variables panel and Waveform/Frequency Spectrum floating surfaces as reference-quality UI
- Avoid regressions in adjacent surfaces that reuse the same shared CSS classes

## Out of Scope

- No product flow changes
- No state/store logic changes unless strictly needed for layout polish
- No workbench model restructuring unless required for a minor styling fix
- No git commit or PR creation in this task

## Acceptance Criteria

- Runtime and Scripts visually feel aligned with Variables and Waveform control surfaces
- Tab styling is cleaner and selected/idle states are easier to scan
- Runtime and Scripts show fewer spacing and hierarchy inconsistencies
- Medium/narrow widths no longer produce obvious grid crowding or broken composition
- `npm --prefix ui run build` passes
