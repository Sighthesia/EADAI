# Task: imu-motion-trajectory-zoom

## Overview

Enhance the IMU workspace so the stage can visualize accelerometer-driven motion trails, support user-controlled zooming, and keep older trajectory segments visible with axis-matched line colors. The result should make short-term motion direction and movement history readable without leaving the existing IMU panel workflow.

## Requirements

- Add a motion trajectory layer to the IMU stage based on the mapped accelerometer channels.
- Preserve historical trajectory segments so users can distinguish the latest path from older path data.
- Render historical trajectory lines with the same per-axis colors used by the mapped accelerometer channels.
- Add IMU stage zoom controls and wheel-based zoom interaction.
- Keep zoom behavior smooth, clamped, and visually consistent with the existing waveform interaction model.
- Bound retained trajectory history so the panel does not grow unbounded during long sessions.

## Acceptance Criteria

- [ ] When accel X/Y/Z channels are mapped and updating, the IMU panel shows a visible motion trajectory overlay on the stage.
- [ ] Older trajectory segments remain visible and use the corresponding accelerometer channel colors rather than a single neutral color.
- [ ] Users can zoom the IMU stage with explicit UI controls and pointer wheel interaction.
- [ ] Zoom level is clamped to a safe range and does not break the stage layout, HUD, or floating controls.
- [ ] The IMU panel still works when mapping is incomplete, showing a safe empty or degraded state instead of crashing.
- [ ] Trajectory history is bounded to a reasonable rolling window or sample count so the UI remains responsive.
- [ ] `npm --prefix ui run build` passes after the change.

## Technical Notes

- Reuse the existing IMU panel surface and rendering helpers instead of creating a separate IMU page.
- Reuse the waveform panel's zoom interaction pattern as the UX reference, but keep IMU zoom state scoped to the IMU view.
- Prefer deriving trajectory state from the existing variable sample history and mapped accel channels instead of introducing a new backend contract.
- Use the app store's stable channel color selection so trajectory colors stay aligned with accelerometer channel identity.
- If trajectory projection needs simplification, prioritize a readable 2D stage representation over physically perfect inertial integration.

## Out of Scope

- Adding new Rust or Tauri IPC commands for IMU trajectory rendering.
- Building a physically accurate 3D inertial navigation or dead-reckoning system.
- Adding persistent saved trajectories across app restarts.
- Changing waveform panel behavior outside any shared styling or interaction conventions needed for consistency.
