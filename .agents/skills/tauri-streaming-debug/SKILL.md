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
- React/Zustand selectors used with `useSyncExternalStore` must return stable snapshots; derive arrays/objects in the component with `useMemo` when the selector would otherwise allocate on every render.
- Real-time waveform cursors need all three parts kept in sync: guide line, callout line, and marker dot. If the cursor looks incomplete, inspect whether one part is only updated in the non-animated path or hidden on a stale branch.
- Tauri event listeners and Zustand writes must be batched in `requestAnimationFrame`, especially in React Strict Mode.

## Performance Triage
- When the UI feels slow but `ingestEvents()` and `buildPlotModel()` look cheap, measure panel commit time next. In this repo, the real bottleneck can be a render-heavy panel such as `ConsolePanel`, not the data ingest path.
- Docked tabs can keep mounting and re-rendering even when they are not the active view. Use `TabNode.isVisible()` and the FlexLayout `visibility` event to gate expensive panels instead of assuming inactive tabs are dormant.
- Console performance scales with DOM size and per-row decode work. Keep the visible console window bounded, memoize row components, and isolate expensive BMI088 decode/formatting logic inside each row.
- `VariablesPanel` performance scales with list width and repeated per-row lookups. Precompute selected-channel membership and channel colors once per render, then pass stable row data into memoized row components.
- Use lightweight dev-only instrumentation first: React `Profiler` for commit cost and `createDevTimingLogger()` for function timing. That combination is usually enough before reaching for a third-party profiling package.

## Recent Lessons
- Treat panel rendering as the primary suspect when the data path is already cheap. The fastest wins came from shrinking the render surface, not from deeper store changes.
- Keep the fake-stream path intact while optimizing. It is the quickest way to reproduce panel pressure and verify that the UI still stays responsive under load.
- Apply memoization where the row count is unbounded or grows with time. Bounded windows and stable row props are safer than trying to optimize every derived value in the store.
- Use the profiler output to decide where to cut work next: `ConsolePanel` first when logs are dense, `VariablesPanel` when channel lists are wide, and `WaveformPanel` only after the other two are clean.

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
- If a panel remains expensive after data-path profiling, trim the render surface first: limit visible rows, memoize row components, and avoid recomputing derived values for items that did not change.

## Verification
- `cargo check` in `src-tauri/`
- `npm run build` in `ui/`
- Start the desktop app and confirm the default fake stream auto-connects, waveforms remain visible, and zooming the time window does not freeze the UI.
- When chasing perf regressions, compare `panel-render ...` logs against `appStore.ingestEvents` and `WaveformPanel.buildPlotModel`; the slowest panel is often the real culprit.
- After a tuning pass, recheck the profiler around the slowest panel before expanding the scope. If the hotspot moved, optimize the new bottleneck instead of layering more store logic.

## References
- `src-tauri/src/fake_session.rs`
- `ui/src/App.tsx`
- `ui/src/store/appStore.ts`
- `ui/src/components/WaveformPanel.tsx`
- `ui/src/components/Workbench.tsx`
- `ui/src/components/ConsolePanel.tsx`
- `ui/src/components/VariablesPanel.tsx`
- `ui/src/lib/logger.ts`
