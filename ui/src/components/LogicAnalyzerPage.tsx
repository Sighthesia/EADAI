import { useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from 'react'
import { useAppStore } from '../store/appStore'
import type { LogicAnalyzerCaptureResult } from '../types'
import {
  buildLogicAnalyzerCursorOverlayItems,
  type LogicAnalyzerCursorOverlayItem,
  type LogicAnalyzerCursorSource,
} from '../lib/logicAnalyzerCursorOverlay'

const EMPTY_CHANNEL_LABELS = ['D0', 'D1', 'D2', 'D3', 'D4', 'D5', 'D6', 'D7']
const LOGIC_SAMPLE_WIDTH = 12
const LOGIC_LANE_HEIGHT = 40
const LOGIC_HIGH_Y = 9
const LOGIC_LOW_Y = 31
export function LogicAnalyzerPage() {
  const logicAnalyzer = useAppStore((state) => state.logicAnalyzer)
  const logicAnalyzerConfig = useAppStore((state) => state.logicAnalyzerConfig)
  const toggleLogicAnalyzerChannel = useAppStore((state) => state.toggleLogicAnalyzerChannel)
  const [menuOpen, setMenuOpen] = useState(true)
  const waveformSectionRef = useRef<HTMLDivElement | null>(null)
  const lastFocusedCaptureRef = useRef<string | null>(null)

  const waveform = logicAnalyzer.lastCapture
  const visibleChannelLabels = logicAnalyzerConfig.selectedChannelLabels
  const backendReadyLabel = logicAnalyzer.executable === 'dev-simulator' ? 'dev simulator ready' : logicAnalyzer.available ? 'sigrok ready' : 'sigrok unavailable'
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
                    {backendReadyLabel}
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
  const stageRef = useRef<HTMLDivElement | null>(null)
  const trackRefs = useRef(new Map<string, HTMLDivElement | null>())
  const channels = useMemo(
    () =>
      visibleChannelLabels.length > 0
        ? capture.channels.filter((channel) => visibleChannelLabels.includes(channel.label))
        : capture.channels,
    [capture.channels, visibleChannelLabels],
  )
  const sampleIndices = useMemo(() => buildSampleIndices(capture.sampleCount, maxSamples), [capture.sampleCount])
  const [cursorSlots, setCursorSlots] = useState<Record<string, number>>({})
  const [overlayItems, setOverlayItems] = useState<LogicAnalyzerCursorOverlayItem[]>([])
  const previewLabel =
    capture.sampleCount > maxSamples
      ? `Previewing ${maxSamples} of ${capture.sampleCount} samples`
      : `Showing ${capture.sampleCount} samples`
  const overlayWidth = stageRef.current?.clientWidth ?? 0
  const overlayHeight = stageRef.current?.clientHeight ?? 0

  useEffect(() => {
    if (channels.length === 0) {
      setCursorSlots({})
      return
    }

    setCursorSlots((current) => {
      const next: Record<string, number> = {}
      for (let index = 0; index < channels.length; index += 1) {
        const channel = channels[index]!
        const fallbackSlot = sampleIndices.length > 1 ? Math.round(((index + 1) / (channels.length + 1)) * (sampleIndices.length - 1)) : 0
        next[channel.label] = clamp(current[channel.label] ?? fallbackSlot, 0, Math.max(0, sampleIndices.length - 1))
      }
      return next
    })
  }, [channels, sampleIndices.length])

  useLayoutEffect(() => {
    const stage = stageRef.current
    if (!stage) {
      return
    }

    const syncOverlay = () => {
      const stageRect = stage.getBoundingClientRect()
      const items: LogicAnalyzerCursorSource[] = []

      for (let index = 0; index < channels.length; index += 1) {
        const channel = channels[index]!
        const track = trackRefs.current.get(channel.label)
        if (!track) {
          continue
        }

        const lane = track.querySelector<HTMLElement>('.logic-waveform-lane')
        if (!lane) {
          continue
        }

        const laneRect = lane.getBoundingClientRect()
        const mapped = sampleIndices.map((sampleIndex) => channel.samples[sampleIndex] ?? null)
        const activeSlot = clamp(cursorSlots[channel.label] ?? 0, 0, Math.max(0, mapped.length - 1))
        const activeValue = mapped[activeSlot] ?? null
        const laneRelativeLeft = laneRect.left - stageRect.left
        const laneRelativeTop = laneRect.top - stageRect.top
        const anchorX = laneRelativeLeft + (mapped.length > 1 ? (activeSlot / (mapped.length - 1)) * laneRect.width : 0)
        const anchorY = laneRelativeTop + (activeValue === true ? 9 : activeValue === false ? 31 : laneRect.height / 2)
        const sampleIndex = sampleIndices[activeSlot] ?? activeSlot
        const currentValue = formatLogicLevel(activeValue)

        items.push({
          key: channel.label,
          label: `${channel.label} · S${sampleIndex} · ${currentValue}`,
          anchorX,
          anchorY,
          laneRect: {
            left: laneRelativeLeft,
            top: laneRelativeTop,
            width: laneRect.width,
            height: laneRect.height,
          },
          sampleText: `S${sampleIndex}`,
          accentColor: '#56dfa1',
        })
      }

      setOverlayItems(buildLogicAnalyzerCursorOverlayItems({ width: stageRect.width, height: stageRect.height }, items))
    }

    syncOverlay()

    const resizeObserver = new ResizeObserver(syncOverlay)
    resizeObserver.observe(stage)
    for (const track of trackRefs.current.values()) {
      if (track) {
        resizeObserver.observe(track)
      }
    }

    window.addEventListener('resize', syncOverlay)
    stage.addEventListener('scroll', syncOverlay, { passive: true })

    return () => {
      resizeObserver.disconnect()
      window.removeEventListener('resize', syncOverlay)
      stage.removeEventListener('scroll', syncOverlay)
    }
  }, [channels, cursorSlots, sampleIndices])

  return (
    <div ref={stageRef} className="logic-waveform-stage">
      <div className="logic-time-axis logic-time-axis-stage">
        <span>{previewLabel}</span>
        <span>{formatSampleRate(capture.sampleRateHz)}</span>
        <span>{formatCaptureDuration(capture.sampleCount, capture.sampleRateHz)}</span>
      </div>

      {channels.length > 0 ? (
        <div className="logic-waveform-list">
          {channels.map((channel, index) => (
            <div
              key={channel.label}
              ref={(element) => {
                if (element) {
                  trackRefs.current.set(channel.label, element)
                } else {
                  trackRefs.current.delete(channel.label)
                }
              }}
              className="logic-waveform-track logic-waveform-track-stage"
            >
              <div className="logic-waveform-label">
                <strong>{channel.label}</strong>
                <small>CH {index + 1}</small>
              </div>
              <div className="logic-waveform-lane" aria-label={`${channel.label} digital waveform`}>
                <DigitalWaveformLane
                  samples={channel.samples}
                  sampleIndices={sampleIndices}
                  onActiveSlotChange={(slot) =>
                    setCursorSlots((current) => (current[channel.label] === slot ? current : { ...current, [channel.label]: slot }))
                  }
                />
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="logic-empty-state">No visible channels selected for the current capture.</div>
      )}

      <div className="logic-cursor-overlay-layer" aria-hidden="true">
        <svg className="logic-cursor-overlay" viewBox={`0 0 ${overlayWidth} ${overlayHeight}`} preserveAspectRatio="none">
          {overlayItems.map((item) => (
            <g key={`${item.key}-overlay`}>
              <line
                className="logic-cursor-overlay-guide"
                x1={item.anchorX}
                y1={item.laneRect.top}
                x2={item.anchorX}
                y2={item.laneRect.top + item.laneRect.height}
              />
              {item.showCallout ? (
                <line className="logic-cursor-overlay-callout" x1={item.anchorX} y1={item.anchorY} x2={item.calloutEndX} y2={item.calloutEndY} />
              ) : null}
            </g>
          ))}
        </svg>
        {overlayItems.map((item) => (
          <div
            key={`${item.key}-label`}
            className="logic-cursor-overlay-label"
            style={{
              left: `${item.labelLeft}px`,
              top: `${item.labelTop}px`,
              width: `${item.labelWidth}px`,
              height: `${item.labelHeight}px`,
            }}
          >
            <span>{item.label}</span>
          </div>
        ))}
      </div>

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

function DigitalWaveformLane({
  samples,
  sampleIndices,
  onActiveSlotChange,
}: {
  samples: Array<boolean | null>
  sampleIndices: number[]
  onActiveSlotChange: (slot: number) => void
}) {
  if (sampleIndices.length === 0) {
    return <span className="logic-waveform-empty">No data</span>
  }

  const mapped = sampleIndices.map((index) => samples[index] ?? null)
  const sampleWidth = LOGIC_SAMPLE_WIDTH
  const laneHeight = LOGIC_LANE_HEIGHT
  const laneWidth = Math.max(sampleWidth, mapped.length * sampleWidth)
  const traces = buildDigitalTracePaths(mapped, sampleWidth, LOGIC_HIGH_Y, LOGIC_LOW_Y)
  const gaps = buildGapRects(mapped, sampleWidth)

  const handlePointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect()
    if (bounds.width <= 0 || mapped.length === 0) {
      return
    }

    const relativeX = clamp(event.clientX - bounds.left, 0, bounds.width)
    const nextSlot = Math.round((relativeX / bounds.width) * (mapped.length - 1))
    onActiveSlotChange(nextSlot)
  }

  return (
    <div className="logic-waveform-lane-hitbox" onPointerMove={handlePointerMove}>
      <svg className="logic-waveform-svg" viewBox={`0 0 ${laneWidth} ${laneHeight}`} preserveAspectRatio="none" aria-hidden="true">
        <line className="logic-waveform-rail" x1={0} y1={LOGIC_HIGH_Y} x2={laneWidth} y2={LOGIC_HIGH_Y} />
        <line className="logic-waveform-rail" x1={0} y1={LOGIC_LOW_Y} x2={laneWidth} y2={LOGIC_LOW_Y} />
        {gaps.map((gap, index) => (
          <rect
            key={`gap-${index}-${gap.x}`}
            className="logic-waveform-gap"
            x={gap.x}
            y={4}
            width={gap.width}
            height={laneHeight - 8}
            rx={4}
            ry={4}
          />
        ))}
        {traces.map((trace, index) => (
          <path key={`trace-${index}`} className="logic-waveform-trace" d={trace} />
        ))}
      </svg>
    </div>
  )
}

function buildDigitalTracePaths(samples: Array<boolean | null>, sampleWidth: number, highY: number, lowY: number) {
  const paths: string[] = []

  for (let index = 0; index < samples.length; ) {
    const value = samples[index]
    if (value === null) {
      index += 1
      continue
    }

    let path = `M ${index * sampleWidth} ${value ? highY : lowY}`
    let previousValue = value
    let cursor = index + 1

    while (cursor < samples.length && samples[cursor] !== null) {
      const nextValue = samples[cursor] === true
      const nextX = cursor * sampleWidth
      path += ` H ${nextX}`
      if (nextValue !== previousValue) {
        path += ` V ${nextValue ? highY : lowY}`
      }
      previousValue = nextValue
      cursor += 1
    }

    path += ` H ${cursor * sampleWidth}`
    paths.push(path)
    index = cursor
  }

  return paths
}

function buildGapRects(samples: Array<boolean | null>, sampleWidth: number) {
  const rects: Array<{ x: number; width: number }> = []

  for (let index = 0; index < samples.length; ) {
    if (samples[index] !== null) {
      index += 1
      continue
    }

    const start = index
    while (index < samples.length && samples[index] === null) {
      index += 1
    }

    rects.push({ x: start * sampleWidth, width: Math.max(sampleWidth, (index - start) * sampleWidth) })
  }

  return rects
}


function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
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

function formatLogicLevel(value: boolean | null) {
  if (value === true) {
    return 'High'
  }

  if (value === false) {
    return 'Low'
  }

  return 'Gap'
}
