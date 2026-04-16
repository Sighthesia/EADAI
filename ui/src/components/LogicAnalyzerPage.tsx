import { useEffect, useMemo, useRef, useState } from 'react'
import { useAppStore } from '../store/appStore'
import type { LogicAnalyzerCaptureResult } from '../types'

const EMPTY_CHANNEL_LABELS = ['D0', 'D1', 'D2', 'D3', 'D4', 'D5', 'D6', 'D7']

export function LogicAnalyzerPage() {
  const logicAnalyzer = useAppStore((state) => state.logicAnalyzer)
  const logicAnalyzerConfig = useAppStore((state) => state.logicAnalyzerConfig)
  const toggleLogicAnalyzerChannel = useAppStore((state) => state.toggleLogicAnalyzerChannel)
  const [menuOpen, setMenuOpen] = useState(true)
  const waveformSectionRef = useRef<HTMLDivElement | null>(null)
  const lastFocusedCaptureRef = useRef<string | null>(null)

  const waveform = logicAnalyzer.lastCapture
  const visibleChannelLabels = logicAnalyzerConfig.selectedChannelLabels
  const emptyChannelLabels = useMemo(
    () =>
      visibleChannelLabels.length > 0
        ? visibleChannelLabels
        : logicAnalyzer.devices[0]?.channels.length
          ? logicAnalyzer.devices[0].channels
          : EMPTY_CHANNEL_LABELS,
    [logicAnalyzer.devices, visibleChannelLabels],
  )

  useEffect(() => {
    if (!waveform) {
      lastFocusedCaptureRef.current = null
      return
    }

    const nextKey = `${waveform.outputPath}:${waveform.capturedAtMs}`
    if (lastFocusedCaptureRef.current === nextKey) {
      return
    }

    lastFocusedCaptureRef.current = nextKey
    window.requestAnimationFrame(() => {
      waveformSectionRef.current?.focus({ preventScroll: true })
    })
  }, [waveform])

  const visibleChannels = useMemo(() => {
    if (!waveform) {
      return []
    }

    return visibleChannelLabels.length > 0
      ? waveform.channels.filter((channel) => visibleChannelLabels.includes(channel.label))
      : waveform.channels
  }, [visibleChannelLabels, waveform])

  return (
    <section className="panel logic-analyzer-stage-panel">
      <div ref={waveformSectionRef} tabIndex={-1} className="logic-stage-shell">
        <div className="logic-stage-surface">
          {waveform ? (
            <DigitalWaveformView capture={waveform} visibleChannelLabels={visibleChannelLabels} />
          ) : (
              <LogicAnalyzerEmptyState
                sessionState={logicAnalyzer.sessionState}
                deviceCount={logicAnalyzer.devices.length}
                selectedChannelLabels={visibleChannelLabels}
                emptyChannelLabels={emptyChannelLabels}
                onToggleChannel={toggleLogicAnalyzerChannel}
              />
          )}
        </div>

        <div className="logic-stage-hud">
          <div className="logic-stage-meta">
            <strong>Logic Analyzer</strong>
            <span>{logicAnalyzer.sessionState}</span>
            <small>
              {waveform
                ? `${waveform.sampleCount} samples · ${visibleChannels.length}/${waveform.channels.length} channels visible`
                : 'Waiting for capture result'}
            </small>
          </div>
          {logicAnalyzer.lastCapture ? (
            <div className="logic-stage-meta logic-stage-meta-secondary">
              <strong>Latest Capture</strong>
              <span>{formatSampleRate(logicAnalyzer.lastCapture.sampleRateHz)}</span>
              <small>{formatCaptureDuration(logicAnalyzer.lastCapture.sampleCount, logicAnalyzer.lastCapture.sampleRateHz)}</small>
            </div>
          ) : null}
        </div>

        <aside className={`logic-floating-menu ${menuOpen ? 'open' : 'collapsed'}`}>
          <div className="logic-floating-top">
            {menuOpen ? (
              <div className="logic-floating-heading">
                <strong>Logic View Controls</strong>
                <small>Connection and capture controls moved to the Connection panel.</small>
              </div>
            ) : null}
            <button type="button" className="ghost-button logic-floating-toggle" onClick={() => setMenuOpen((value) => !value)}>
              {menuOpen ? 'Hide' : 'Logic'}
            </button>
          </div>

          {menuOpen ? (
            <div className="logic-floating-scroll">
              <section className="logic-floating-section">
                <div className="logic-floating-section-header">
                  <strong>Capture Summary</strong>
                  <small>{logicAnalyzer.executable ?? 'sigrok-cli unresolved'}</small>
                </div>
                <div className="logic-floating-chip-row">
                  <span className={`status-pill tone-${logicAnalyzer.available ? 'success' : 'warning'}`}>
                    {logicAnalyzer.available ? 'sigrok ready' : 'sigrok unavailable'}
                  </span>
                  <span className="imu-chip">{logicAnalyzer.devices.length} devices</span>
                  <span className="imu-chip">{logicAnalyzer.sessionState}</span>
                </div>
                <p className="logic-floating-copy">{logicAnalyzer.lastError ?? logicAnalyzer.linuxFirstNote}</p>
              </section>

              <section className="logic-floating-section">
                <div className="logic-floating-section-header">
                  <strong>Visible Channels</strong>
                  <small>
                    {waveform
                      ? `${visibleChannels.length} of ${waveform.channels.length} visible`
                      : `${emptyChannelLabels.length} ready before capture`}
                  </small>
                </div>
                {waveform ? (
                  <div className="logic-chip-row">
                    {waveform.channels.map((channel) => {
                      const selected = visibleChannelLabels.includes(channel.label)
                      return (
                        <button
                          key={channel.label}
                          type="button"
                          className={`logic-channel-chip ${selected ? 'selected' : ''}`}
                          onClick={() => toggleLogicAnalyzerChannel(channel.label)}
                        >
                          <span className="logic-chip-mark" aria-hidden="true">
                            {selected ? '✓' : '+'}
                          </span>
                          {channel.label}
                        </button>
                      )
                    })}
                  </div>
                ) : (
                  <div className="logic-chip-row">
                    {emptyChannelLabels.map((label) => {
                      const selected = visibleChannelLabels.includes(label)
                      return (
                        <button
                          key={label}
                          type="button"
                          className={`logic-channel-chip ${selected ? 'selected' : ''}`}
                          onClick={() => toggleLogicAnalyzerChannel(label)}
                        >
                          <span className="logic-chip-mark" aria-hidden="true">
                            {selected ? '✓' : '+'}
                          </span>
                          {label}
                        </button>
                      )
                    })}
                  </div>
                )}
              </section>

              <section className="logic-floating-section">
                <div className="logic-floating-section-header">
                  <strong>Capture Plan</strong>
                  <small>Sidecar boundary</small>
                </div>
                <pre className="logic-pre">{logicAnalyzer.capturePlan ?? 'No capture plan generated yet.'}</pre>
              </section>
            </div>
          ) : null}
        </aside>

        {!waveform ? (
          <div className="logic-bottom-banner" role="status">
            <strong>Logic analyzer not connected</strong>
            <span>Use the Connection panel to scan devices and start a capture. Tracks stay ready here.</span>
          </div>
        ) : null}
      </div>
    </section>
  )
}

function LogicAnalyzerEmptyState({
  sessionState,
  deviceCount,
  selectedChannelLabels,
  emptyChannelLabels,
  onToggleChannel,
}: {
  sessionState: string
  deviceCount: number
  selectedChannelLabels: string[]
  emptyChannelLabels: string[]
  onToggleChannel: (channel: string) => void
}) {
  return (
    <div className="logic-empty-stage">
      <div className="logic-time-axis logic-time-axis-stage logic-time-axis-empty">
        <span>Armed view</span>
        <span>{deviceCount > 0 ? `${deviceCount} devices detected` : 'No device selected yet'}</span>
        <span>{sessionState}</span>
      </div>

      <div className="logic-empty-track-list">
        {emptyChannelLabels.map((label, index) => {
          const selected = selectedChannelLabels.includes(label)
          return (
            <div key={label} className={`logic-waveform-track logic-waveform-track-stage logic-waveform-track-placeholder ${selected ? 'selected' : ''}`}>
              <div className="logic-waveform-label logic-waveform-label-placeholder">
                <strong>{label}</strong>
                <small>CH {index + 1}</small>
              </div>
              <div className="logic-waveform-lane logic-waveform-lane-placeholder">
                <button
                  type="button"
                  className={`logic-track-arm ${selected ? 'selected' : ''}`}
                  onClick={() => onToggleChannel(label)}
                >
                  <span className="logic-track-arm-icon" aria-hidden="true">
                    {selected ? '◉' : '○'}
                  </span>
                  <span>{selected ? 'Visible' : 'Hidden'}</span>
                </button>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

function DigitalWaveformView({
  capture,
  visibleChannelLabels,
}: {
  capture: LogicAnalyzerCaptureResult
  visibleChannelLabels: string[]
}) {
  const maxSamples = 96
  const channels =
    visibleChannelLabels.length > 0
      ? capture.channels.filter((channel) => visibleChannelLabels.includes(channel.label))
      : capture.channels
  const sampleIndices = buildSampleIndices(capture.sampleCount, maxSamples)
  const previewLabel =
    capture.sampleCount > maxSamples
      ? `Previewing ${maxSamples} of ${capture.sampleCount} samples`
      : `Showing ${capture.sampleCount} samples`

  return (
    <div className="logic-waveform-stage">
      <div className="logic-time-axis logic-time-axis-stage">
        <span>{previewLabel}</span>
        <span>{formatSampleRate(capture.sampleRateHz)}</span>
        <span>{formatCaptureDuration(capture.sampleCount, capture.sampleRateHz)}</span>
      </div>

      {channels.length > 0 ? (
        <div className="logic-waveform-list">
          {channels.map((channel, index) => (
            <div key={channel.label} className="logic-waveform-track logic-waveform-track-stage">
              <div className="logic-waveform-label">
                <strong>{channel.label}</strong>
                <small>CH {index + 1}</small>
              </div>
              <div className="logic-waveform-lane" aria-label={`${channel.label} digital waveform`}>
                {renderDigitalLane(channel.samples, sampleIndices)}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="logic-empty-state">No visible channels selected for the current capture.</div>
      )}

      <small className="logic-waveform-footnote">
        Simplified staircase view from captured CSV text; null samples are treated as gaps.
      </small>
    </div>
  )
}

function buildSampleIndices(sampleCount: number, maxSamples: number) {
  if (sampleCount <= maxSamples) {
    return Array.from({ length: sampleCount }, (_, index) => index)
  }

  const step = Math.max(1, Math.floor(sampleCount / maxSamples))
  const indices: number[] = []
  for (let index = 0; index < sampleCount; index += step) {
    indices.push(index)
  }
  return indices.slice(0, maxSamples)
}

function renderDigitalLane(samples: Array<boolean | null>, sampleIndices: number[]) {
  if (sampleIndices.length === 0) {
    return <span className="logic-waveform-empty">No data</span>
  }

  const mapped = sampleIndices.map((index) => samples[index] ?? null)

  return (
    <div className="logic-waveform-segments">
      {mapped.map((value, index) => (
        <span
          key={`${index}-${value === null ? 'gap' : value ? 'high' : 'low'}`}
          className={`logic-waveform-segment ${value === null ? 'gap' : value ? 'high' : 'low'}`}
          title={value === null ? 'gap' : value ? 'high' : 'low'}
        />
      ))}
    </div>
  )
}

function formatSampleRate(sampleRateHz?: number | null) {
  if (!sampleRateHz || sampleRateHz <= 0) {
    return 'Sample rate unavailable'
  }

  if (sampleRateHz >= 1_000_000) {
    return `${(sampleRateHz / 1_000_000).toFixed(sampleRateHz % 1_000_000 === 0 ? 0 : 2)} MHz`
  }

  if (sampleRateHz >= 1_000) {
    return `${(sampleRateHz / 1_000).toFixed(sampleRateHz % 1_000 === 0 ? 0 : 2)} kHz`
  }

  return `${sampleRateHz} Hz`
}

function formatCaptureDuration(sampleCount: number, sampleRateHz?: number | null) {
  if (!sampleRateHz || sampleRateHz <= 0) {
    return 'Capture span unavailable'
  }

  const durationMs = (sampleCount / sampleRateHz) * 1000
  return `Span ${durationMs.toFixed(durationMs >= 10 ? 2 : 3)} ms`
}
