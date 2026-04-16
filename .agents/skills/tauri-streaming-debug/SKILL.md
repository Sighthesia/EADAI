---
name: tauri-streaming-debug
description: Use when working on the Tauri serial workbench, especially if fake streams show empty waveforms, high CPU, or UI freezes during continuous updates.
---

# Tauri Streaming Debug

The Tauri workbench stays responsive only when serial events are timestamp-aligned, batched before entering the store, and rendered through persistent chart instances.

## When to Use
- Adding or changing fake serial sources for frontend debugging
- Investigating waveforms that only show axes or become sparse/invisible
- Investigating Tauri UI freezes after several seconds of continuous streaming
- Changing how serial bus events are forwarded into the React store

## Symptoms
- Waveform axes move, but lines or points are missing
- The UI becomes noticeably slower after 10-20 seconds of fake streaming
- React dev mode appears to duplicate or amplify serial updates
- uPlot re-renders too often when new samples arrive

## Root Cause
- Fake multi-channel samples must share the same logical timestamp, otherwise `WaveformPanel` aligns channels with many `null` gaps and lines disappear.
- The store must prefer parser-provided `timestamp` over per-event wall-clock time when building sample points.
- `uPlot` must be created once per series structure and updated with `setData()` / `setSize()` instead of `destroy() + new` for each sample.
- Tauri event listeners and Zustand writes must be batched in `requestAnimationFrame`, especially in React Strict Mode.

## Correct Pattern
- Keep fake source emission in `src-tauri/src/fake_session.rs` and emit one shared `timestamp=<ms>` field for every metric in the same batch.
- Batch frontend bus events in `ui/src/App.tsx` before calling the store.
- Process grouped events in `ui/src/store/appStore.ts::ingestEvents()` so one animation frame produces one Zustand update.
- In `ui/src/components/WaveformPanel.tsx`, keep a rolling time window, normalize x-values to visible seconds, and update the existing uPlot instance instead of rebuilding it.
- Overlay labels should use a two-line card format: first line is the variable name, second line is the latest value or cursor value; do not split name and value into separate floating widgets.
- Visual measurement and IMU sections in `ui/src/components/VariablesPanel.tsx` should be grouped into collapsible blocks, and each block should contain an internal scrollbox when content can grow beyond the visible menu height.
- Context menus that contain internal scrollboxes must stop `pointerdown`, `wheel`, and `scroll` propagation on the menu root so scrolling the menu does not close it or fall through to the underlying card list.
- If a menu still closes on internal scrolling, do not rely only on propagation stopping: close the menu only when the external event target is outside the menu root, and add `overscroll-behavior: contain` to the menu scrollbox.
- If performance degrades again, inspect console scrolling and chip list growth before blaming the parser.

## Verification
- `cargo check` in `src-tauri/`
- `npm run build` in `ui/`
- Start the desktop app and confirm the default fake stream auto-connects, waveforms remain visible, and zooming the time window does not freeze the UI.

## References
- `src-tauri/src/fake_session.rs`
- `ui/src/App.tsx`
- `ui/src/store/appStore.ts`
- `ui/src/components/WaveformPanel.tsx`
