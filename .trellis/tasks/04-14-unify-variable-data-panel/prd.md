# Simplify IMU Tab Layout

## Goal
Refocus the IMU tab around the 3D attitude renderer so the visualization becomes the primary surface, while all tunable controls move into a compact floating menu on the right.

## Requirements
- Let the IMU 3D render surface occupy the full IMU tab instead of sharing space with multiple dense status cards.
- Reduce visible information density by removing non-essential always-on metric cards from the main layout.
- Move adjustable IMU parameters into a right-side floating panel with semi-transparent styling.
- Keep orientation source switching, channel mapping, auto-detect, and calibration actions available without leaving the IMU tab.
- Preserve the existing IMU data flow and mapping behavior in the store.

## Acceptance Criteria
- [ ] The IMU tab renders as one full-size visualization surface.
- [ ] A semi-transparent floating control menu appears on the right side of the IMU tab.
- [ ] The floating menu exposes source selection, mapping controls, auto-detect, and calibration actions.
- [ ] The previous multi-card IMU dashboard no longer dominates the tab layout.
- [ ] The frontend build passes after the layout refactor.

## Notes
This change is intentionally visual and structural only. It should not alter IMU parsing, orientation derivation, or state management contracts.
