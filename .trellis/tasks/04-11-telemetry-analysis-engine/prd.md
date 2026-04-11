# Telemetry Analysis Engine for Waveform Metrics and Triggers

## 1. Purpose

Build a reusable analysis layer on top of the current fake/real serial workbench so the app can compute waveform-derived metrics, trigger scripted actions, and provide AI-ready structured telemetry.

This task should produce an MVP that is stable enough for frontend and AI integration, while keeping the analysis pipeline small, bounded, and easy to extend.

## 2. Why This Matters

- The workbench currently shows raw lines and live plots, but does not yet derive higher-level facts from them.
- AI features need stable structured features, not only unparsed serial text.
- User workflows will improve once the app can answer: frequency, period, duty cycle, peak, RMS, min/max, thresholds, and trigger conditions.

## 3. Goals

- Derive waveform metrics from time-series samples.
- Support trigger rules or lightweight scripts based on analysis results.
- Expose analysis results through a stable bus/data contract.
- Keep the raw data path intact so the UI can still inspect source samples.
- Make the output suitable for future AI summarization and recommendations.

## 4. Recommended Baseline

### Data flow

```text
Serial / Fake Source
  -> Framer
  -> Parser
  -> Sample normalizer
  -> Analysis engine
  -> Trigger evaluator / script hook
  -> UI + AI-facing structured stream
```

### Analysis scope for MVP

- Frequency
- Period
- Duty cycle
- Rising/falling edge count
- Min / max
- Mean / RMS
- Windowed trend and change rate

### Trigger scope for MVP

- Threshold trigger
- Range trigger
- Edge-count trigger
- Simple script callback or rule expression

## 5. Non-Goals

- Full general-purpose scripting runtime
- Heavy offline analytics
- Complex machine-learning inference inside the UI thread
- Replacing the current waveform viewer
- Multi-device orchestration

## 6. MVP Deliverables

1. A normalized analysis data model.
2. A windowed metrics calculator for channel samples.
3. A trigger evaluator that emits structured trigger events.
4. A clear contract for AI-facing summaries.
5. Tests for metric math and trigger activation.

## 7. Architecture Notes

### Analysis contract

The analysis layer should operate on stable sample windows, not raw render state.

Logical shape:

```text
analysis_frame = {
  channel_id,
  window_ms,
  sample_count,
  frequency?,
  period?,
  duty_cycle?,
  min?,
  max?,
  mean?,
  rms?,
  edge_count?,
  trigger_hits?
}
```

### Trigger contract

Trigger results should be explicit events, not implicit UI side effects.

```text
trigger_event = {
  channel_id,
  rule_id,
  severity,
  fired_at,
  reason,
  snapshot?
}
```

### AI readiness

AI consumers should receive:

- raw line text
- parser fields
- channel metadata
- windowed metrics
- trigger events
- recent history summary

## 8. Suggested Implementation Boundaries

- Keep transport and framing in the existing Rust serial layer.
- Add analysis as a separate backend module or service.
- Keep visualization components consuming the analysis stream, not computing metrics themselves.
- Keep trigger scripts/rules isolated from the rendering layer.

## 9. Risks

- Incorrect edge detection will produce misleading frequency and duty cycle values.
- UI-thread analysis will cause lag if it is not batched or bounded.
- If the analysis contract is too ad hoc, AI integration will be hard to stabilize later.
- Trigger rules that run too often can flood the UI and logs.

## 10. Acceptance Criteria

- [ ] A sample window can be converted into frequency, duty cycle, and edge metrics.
- [ ] Trigger rules can fire on structured metrics.
- [ ] Analysis results are emitted separately from raw sample events.
- [ ] The UI can consume analysis results without recomputing the math.
- [ ] Tests cover at least one waveform-like pulse stream and one noisy stream.

## 11. Minimum Viable Knowledge

- AI needs stable structured features more than raw text.
- Waveform metrics should be computed over windows, not the full history.
- Trigger output should be a first-class event so it can later drive scripts, alerts, or AI summaries.
