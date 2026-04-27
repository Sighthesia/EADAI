import { useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import uPlot from 'uplot'
import { computeStableCycleStats } from '../lib/waveformPeriod'
import { isWaveformVisualAidEnabled, type WaveformVisualAidState } from '../lib/waveformVisualAids'
import { formatWaveformWindowMs, MAX_WAVEFORM_WINDOW_MS, MIN_WAVEFORM_WINDOW_MS, scaleWaveformWindowMs } from '../lib/waveformWindow'
import { createDevTimingLogger } from '../lib/logger'
import { useAppStore } from '../store/appStore'
import type { SelectedWaveformVariable } from '../store/selectors/waveformSelectors'
import type { UiAnalysisPayload, VariableEntry } from '../types'

const SLOPE_REGRESSION_POINT_COUNT = 8
const CURSOR_LABEL_GAP_PX = 42
const CURSOR_LABEL_MIN_CENTER_Y = 22
const CURSOR_LABEL_MAX_WIDTH_PX = 180
const CURSOR_CALLOUT_THRESHOLD_PX = 1
const CURSOR_SMOOTHING_X = 0.45
const CURSOR_SMOOTHING_Y = 0.2
const CURSOR_SNAP_DISTANCE_PX = 120
const CURSOR_ANIMATION_EPSILON_PX = 0.5
const LATEST_SMOOTHING_X = 1
const LATEST_SMOOTHING_Y = 1
const MAX_CURSOR_LABEL_TRACKS = 4
const MAX_PERSISTENT_LABEL_TRACKS = 8
const OVERLAY_MOTION_DEBUG_STORAGE_KEY = 'eadai:waveform-overlay-motion-debug'
const profileBuildPlotModel = createDevTimingLogger('WaveformPanel.buildPlotModel', { slowThresholdMs: 8, summaryEvery: 120, summaryIntervalMs: 5_000 })

const SVG_NS = 'http://www.w3.org/2000/svg'

type NumericStats = {
  minValue: number
  maxValue: number
  meanValue: number
  medianValue: number
  changeRate?: number | null
}

type PeriodStats = {
  periodMs: number | null
  cycleStartsMs: number[]
  meanValue: number | null
  medianValue: number | null
}

type PlotTrack = {
  name: string
  color: string
  variable: VariableEntry
  labelsEnabled: boolean
  rangeEnabled: boolean
  meanEnabled: boolean
  medianEnabled: boolean
  periodEnabled: boolean
  slopeEnabled: boolean
  points: Array<{ timestampMs: number; value: number }>
  displayValue: string
  seriesIndex: number
  stats: NumericStats
  period: PeriodStats
}

type TextTrack = {
  name: string
  color: string
  textEnabled: boolean
  value: string
  updatedAtMs: number
}

type PlotModel = {
  data: uPlot.AlignedData
  series: uPlot.Series[]
  numericTracks: PlotTrack[]
  textTracks: TextTrack[]
  windowStartMs: number
  windowEndMs: number
  xMin: number
  xMax: number
  yMin: number
  yMax: number
}

export function WaveformPanel() {
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const variables = useAppStore((state) => state.variables)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const visualAidState = useAppStore((state) => state.visualAidState)
  const timeWindowMs = useAppStore((state) => state.waveformWindowMs)
  const setTimeWindowMs = useAppStore((state) => state.setWaveformWindowMs)
  const [menuOpen, setMenuOpen] = useState(true)

  const selectedVariables = useMemo(
    () =>
      selectedChannels.reduce<SelectedWaveformVariable[]>((acc, channel) => {
        const variable = variables[channel]
        if (!variable) {
          return acc
        }

        acc.push({
          name: variable.name,
          color: colorForChannel(variable.name),
          variable,
        })
        return acc
      }, []),
    [colorForChannel, selectedChannels, variables],
  )

  const { numericCount, textCount } = useMemo(() => {
    let numericCount = 0
    for (const item of selectedVariables) {
      if (isNumericVariable(item.variable)) {
        numericCount++
      }
    }

    return {
      numericCount,
      textCount: selectedVariables.length - numericCount,
    }
  }, [selectedVariables])

  return (
    <section className="panel waveform-panel">
      <WavePlot selectedVariables={selectedVariables} visualAidState={visualAidState} timeWindowMs={timeWindowMs} onTimeWindowChange={setTimeWindowMs} />

      <div className="waveform-stage-hud">
        <div className="waveform-stage-meta">
          <strong>Waveforms</strong>
          <span>{selectedVariables.length > 0 ? `${numericCount} waveforms · ${textCount} text tracks` : 'Idle'}</span>
          <small>{formatTimeWindow(timeWindowMs)}</small>
        </div>
      </div>

      <div className={`waveform-floating-menu ${menuOpen ? '' : 'collapsed'}`}>
        <div className="waveform-floating-top">
          {menuOpen ? (
            <div className="waveform-floating-heading">
              <strong>Waveform Controls</strong>
              <small>{selectedVariables.length > 0 ? 'Pan with mouse wheel; tune the overlay controls here.' : 'Select variables to start plotting.'}</small>
            </div>
          ) : null}
          <button type="button" className="ghost-button waveform-floating-toggle" onClick={() => setMenuOpen((value) => !value)}>
            {menuOpen ? 'Hide' : 'Waveforms'}
          </button>
        </div>

        {menuOpen ? (
          <div className="waveform-floating-scroll">
            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Window</strong>
                <small>{formatTimeWindow(timeWindowMs)}</small>
              </div>
              <div className="waveform-controls">
                <button
                  type="button"
                  className="ghost-button waveform-zoom-icon-button"
                  onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 0.8))}
                  aria-label="Zoom in"
                  title="Zoom in"
                >
                  <MagnifyPlusIcon />
                </button>
                <label className="waveform-zoom-label">
                  <span>Visible span</span>
                  <input
                    type="range"
                    min={MIN_WAVEFORM_WINDOW_MS}
                    max={MAX_WAVEFORM_WINDOW_MS}
                    step={1_000}
                    value={timeWindowMs}
                    onChange={(event) => setTimeWindowMs(Number(event.target.value))}
                  />
                </label>
                <button
                  type="button"
                  className="ghost-button waveform-zoom-icon-button"
                  onClick={() => setTimeWindowMs((value) => scaleTimeWindow(value, 1.25))}
                  aria-label="Zoom out"
                  title="Zoom out"
                >
                  <MagnifyMinusIcon />
                </button>
              </div>
            </section>
            <section className="waveform-floating-section">
              <div className="waveform-floating-section-header">
                <strong>Overlays</strong>
                <small>{selectedVariables.length > 0 ? `${numericCount} numeric · ${textCount} text` : 'No active channels'}</small>
              </div>
              <p className="waveform-floating-note">Right-click any variable card to enable or disable max/min, slope, and text overlays.</p>
            </section>
          </div>
        ) : null}
      </div>
    </section>
  )
}

function WavePlot({
  selectedVariables,
  visualAidState,
  timeWindowMs,
  onTimeWindowChange,
}: {
  selectedVariables: SelectedWaveformVariable[]
  visualAidState: WaveformVisualAidState
  timeWindowMs: number
  onTimeWindowChange: (value: number | ((current: number) => number)) => void
}) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const structureKeyRef = useRef('')
  const modelRef = useRef<PlotModel | null>(null)
  const yScaleAnimationRef = useRef<YScaleAnimationState | null>(null)
  const [size, setSize] = useState({ width: 900, height: 440 })
  const model = useMemo(() => buildPlotModel(selectedVariables, visualAidState, timeWindowMs), [selectedVariables, timeWindowMs, visualAidState])
  const structureKey = useMemo(() => (model.numericTracks.length === 0 ? '__empty__' : model.numericTracks.map((item) => item.name).join('|')), [model.numericTracks])

  modelRef.current = model

  useEffect(() => {
    if (!hostRef.current) {
      return
    }

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0]
      if (!entry) {
        return
      }
      setSize({
        width: Math.max(480, Math.floor(entry.contentRect.width)),
        height: Math.max(320, Math.floor(entry.contentRect.height)),
      })
    })
    observer.observe(hostRef.current)

    const wheelTarget = hostRef.current
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      onTimeWindowChange((value) => scaleTimeWindow(value, event.deltaY < 0 ? 0.88 : 1.12))
    }
    wheelTarget.addEventListener('wheel', onWheel, { passive: false })

    return () => {
      observer.disconnect()
      wheelTarget.removeEventListener('wheel', onWheel)
    }
  }, [onTimeWindowChange])

  useEffect(() => {
    if (!hostRef.current) {
      return
    }

    if (!plotRef.current || structureKeyRef.current !== structureKey) {
      plotRef.current?.destroy()
      plotRef.current = new uPlot(
        {
          width: size.width,
          height: size.height,
          padding: [12, 16, 12, 16],
          scales: {
            x: { time: false, auto: false },
            y: { auto: false },
          },
          axes: [
            {
              stroke: '#5f6b7a',
              grid: { stroke: '#20242d' },
              values: (_, ticks) => {
                const visibleSpanSeconds = Math.max(modelRef.current?.xMax ?? model.xMax, 0)
                return ticks.map((tick) => {
                  const elapsedSeconds = Math.max(0, visibleSpanSeconds - tick)
                  return `${elapsedSeconds.toFixed(elapsedSeconds >= 10 ? 0 : 1)}s`
                })
              },
            },
            { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
          ],
          series: model.series,
          plugins: [createMeasurementOverlayPlugin(modelRef)],
        },
        model.data,
        hostRef.current,
      )
      yScaleAnimationRef.current = {
        currentMin: model.yMin,
        currentMax: model.yMax,
        targetMin: model.yMin,
        targetMax: model.yMax,
        frameId: null,
      }
      structureKeyRef.current = structureKey
    }

    return () => {
      if (!hostRef.current?.isConnected) {
        stopYScaleAnimation(yScaleAnimationRef.current)
        plotRef.current?.destroy()
        plotRef.current = null
        yScaleAnimationRef.current = null
        structureKeyRef.current = ''
      }
    }
  }, [model.data, model.series, size, structureKey])

  useEffect(() => {
    plotRef.current?.setSize(size)
  }, [size])

  useEffect(() => {
    if (!plotRef.current) {
      return
    }

    plotRef.current.setData(model.data)
    plotRef.current.setScale('x', { min: model.xMin, max: model.xMax })
    plotRef.current.setScale('y', { min: model.yMin, max: model.yMax })
  }, [model.data, model.xMin, model.xMax, model.yMin, model.yMax])

  useEffect(() => {
    plotRef.current?.redraw()
  }, [visualAidState])

  useEffect(
    () => () => {
      stopYScaleAnimation(yScaleAnimationRef.current)
      plotRef.current?.destroy()
      plotRef.current = null
      yScaleAnimationRef.current = null
      structureKeyRef.current = ''
    },
    [],
  )

  return <div className="wave-plot waveform-stage-surface" ref={hostRef} />
}

function buildPlotModel(selectedVariables: SelectedWaveformVariable[], visualAidState: WaveformVisualAidState, timeWindowMs: number): PlotModel {
  const startedAtMs = performance.now()

  try {
    const stagedVariables = selectedVariables.map(({ name, color, variable }) => ({
      name,
      color,
      variable,
      labelsEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'labels'),
      rangeEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'range'),
      meanEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'mean'),
      medianEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'median'),
      periodEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'period'),
      slopeEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'slope'),
    }))

    let latestPointTimestampMs = Number.NEGATIVE_INFINITY
    let latestUpdateTimestampMs = Number.NEGATIVE_INFINITY
    for (const { variable } of stagedVariables) {
      latestUpdateTimestampMs = Math.max(latestUpdateTimestampMs, variable.updatedAtMs)

      if (isNumericVariable(variable)) {
        const sourcePoints = variable.points.length > 0 ? variable.points : createFallbackNumericPoint(variable, variable.updatedAtMs)
        if (sourcePoints.length > 0) {
          latestPointTimestampMs = Math.max(latestPointTimestampMs, sourcePoints[sourcePoints.length - 1]!.timestampMs)
        }
      }
    }

    const latestTimestampMs = Number.isFinite(latestPointTimestampMs)
      ? latestPointTimestampMs
      : Number.isFinite(latestUpdateTimestampMs)
        ? latestUpdateTimestampMs
        : Date.now()

    const windowEndMs = latestTimestampMs
    let dataWindowStartMs = windowEndMs - timeWindowMs

    for (const item of stagedVariables) {
      if (!isNumericVariable(item.variable) || item.variable.points.length === 0) {
        continue
      }
      const earliest = item.variable.points[0]!.timestampMs
      if (Number.isFinite(earliest) && earliest > dataWindowStartMs) {
        dataWindowStartMs = earliest
      }
    }

    const numericTracks: PlotTrack[] = []
    const textTracks: TextTrack[] = []
    const timestamps = new Set<number>()
    let yMin = Number.POSITIVE_INFINITY
    let yMax = Number.NEGATIVE_INFINITY

    for (const item of stagedVariables) {
      if (isNumericVariable(item.variable)) {
        const sourcePoints = item.variable.points.length > 0 ? item.variable.points : createFallbackNumericPoint(item.variable, windowEndMs)
        const visibleStartIndex = findFirstVisiblePointIndex(sourcePoints, dataWindowStartMs)
        const visiblePoints = visibleStartIndex > 0 ? sourcePoints.slice(visibleStartIndex) : sourcePoints
        const period = computePeriodStats(sourcePoints, item.variable.analysis)
        const stats = resolveNumericStats(item.variable, visiblePoints, sourcePoints, period)
        const rangePoints = visiblePoints.length > 0 ? visiblePoints : sourcePoints

        for (const point of rangePoints) {
          timestamps.add(point.timestampMs)
          yMin = Math.min(yMin, point.value)
          yMax = Math.max(yMax, point.value)
        }

        numericTracks.push({
          name: item.name,
          color: item.color,
          variable: item.variable,
          labelsEnabled: item.labelsEnabled,
          rangeEnabled: item.rangeEnabled,
          meanEnabled: item.meanEnabled,
          medianEnabled: item.medianEnabled,
          periodEnabled: item.periodEnabled,
          slopeEnabled: item.slopeEnabled,
          points: visiblePoints,
          displayValue: formatDisplayValue(item.variable, item.variable.numericValue ?? visiblePoints[visiblePoints.length - 1]?.value ?? sourcePoints[sourcePoints.length - 1]?.value),
          seriesIndex: numericTracks.length + 1,
          stats,
          period,
        })
        continue
      }

      textTracks.push({
        name: item.name,
        color: item.color,
        textEnabled: isWaveformVisualAidEnabled(visualAidState, item.name, 'text'),
        value: item.variable.currentValue,
        updatedAtMs: item.variable.updatedAtMs,
      })
    }

    const sortedTimestamps = Array.from(timestamps).sort((left, right) => left - right)

    if (numericTracks.length === 0 || sortedTimestamps.length === 0) {
      return {
        data: [[0], [0]] as uPlot.AlignedData,
        series: [
          {} as uPlot.Series,
          {
            label: textTracks.length > 0 ? 'Text channels shown in overlay' : 'No numeric channel selected',
            stroke: '#5f6b7a',
            width: 2,
          } as uPlot.Series,
        ] as uPlot.Series[],
        numericTracks,
        textTracks,
        windowStartMs: dataWindowStartMs,
        windowEndMs,
        xMin: 0,
        xMax: 1,
        yMin: 0,
        yMax: 1,
      }
    }

    const plotStartMs = sortedTimestamps[0] ?? dataWindowStartMs
    const normalizedTimestamps = sortedTimestamps.map((timestamp) => (timestamp - plotStartMs) / 1000)
    const plotSpanSeconds = Math.max((windowEndMs - plotStartMs) / 1000, 1)
    const showPoints = normalizedTimestamps.length <= 240
    const data: Array<number[] | Array<number | null>> = [normalizedTimestamps]
    const plotSeries = [{ label: 'time' } as uPlot.Series]

    for (const item of numericTracks) {
      const row = buildAlignedRow(item.points, sortedTimestamps)
      data.push(row)
      plotSeries.push({
        label: item.name,
        stroke: item.color,
        width: 2,
        points: { show: showPoints, size: 4, width: 1 },
      } as uPlot.Series)
    }

    const { min: yMinFinal, max: yMaxFinal } = finalizeYRange(yMin, yMax)

    return {
      data: data as unknown as uPlot.AlignedData,
      series: plotSeries,
      numericTracks,
      textTracks,
      windowStartMs: plotStartMs,
      windowEndMs,
      xMin: 0,
      xMax: plotSpanSeconds,
      yMin: yMinFinal,
      yMax: yMaxFinal,
    }
  } finally {
    profileBuildPlotModel(performance.now() - startedAtMs, {
      selectedCount: selectedVariables.length,
      windowMs: timeWindowMs,
    })
  }
}

function scaleTimeWindow(current: number, factor: number) {
  return clampTimeWindow(Math.round(current * factor))
}

function clampTimeWindow(value: number) {
  return Math.min(MAX_WAVEFORM_WINDOW_MS, Math.max(MIN_WAVEFORM_WINDOW_MS, value))
}

function isNumericVariable(variable: VariableEntry) {
  if (Number.isFinite(variable.numericValue)) {
    return true
  }
  if (variable.points.length > 0) {
    return true
  }
  return hasNumericAnalysis(variable.analysis)
}

function hasNumericAnalysis(analysis?: UiAnalysisPayload | null) {
  if (!analysis) {
    return false
  }

  return [analysis.minValue, analysis.maxValue, analysis.meanValue, analysis.medianValue, analysis.changeRate].some((value) => Number.isFinite(value ?? Number.NaN))
}

function createFallbackNumericPoint(variable: VariableEntry, timestampMs: number) {
  if (!Number.isFinite(variable.numericValue)) {
    return []
  }

  return [{ timestampMs, value: variable.numericValue as number }]
}

function findFirstVisiblePointIndex(points: Array<{ timestampMs: number }>, windowStartMs: number) {
  let low = 0
  let high = points.length

  while (low < high) {
    const mid = (low + high) >>> 1
    if (points[mid]!.timestampMs < windowStartMs) {
      low = mid + 1
    } else {
      high = mid
    }
  }

  return low
}

function buildAlignedRow(points: Array<{ timestampMs: number; value: number }>, timestamps: number[]) {
  const row: Array<number | null> = new Array(timestamps.length)
  let pointIndex = 0

  for (let index = 0; index < timestamps.length; index += 1) {
    const timestampMs = timestamps[index]!
    while (pointIndex < points.length && points[pointIndex]!.timestampMs < timestampMs) {
      pointIndex += 1
    }

    row[index] = points[pointIndex]?.timestampMs === timestampMs ? points[pointIndex]!.value : null
  }

  return row
}

function finalizeYRange(min: number, max: number) {
  if (!Number.isFinite(min) || !Number.isFinite(max)) {
    return { min: 0, max: 1 }
  }

  if (min === max) {
    const padding = Math.max(Math.abs(min) * 0.08, 1)
    return { min: min - padding, max: max + padding }
  }

  const span = max - min
  const padding = Math.max(span * 0.08, 0.5)
  const nextMin = min - padding
  const nextMax = max + padding

  if (nextMin === nextMax) {
    return { min: nextMin - 1, max: nextMax + 1 }
  }

  return { min: nextMin, max: nextMax }
}

function syncAnimatedYScale(
  plot: uPlot,
  stateRef: MutableRefObject<YScaleAnimationState | null>,
  nextMin: number,
  nextMax: number,
) {
  const current = stateRef.current
  plot.setScale('y', { min: nextMin, max: nextMax })
  if (!current) {
    stateRef.current = {
      currentMin: nextMin,
      currentMax: nextMax,
      targetMin: nextMin,
      targetMax: nextMax,
      frameId: null,
    }
    return
  }

  current.currentMin = nextMin
  current.currentMax = nextMax
  current.targetMin = nextMin
  current.targetMax = nextMax
}

function stopYScaleAnimation(_state: YScaleAnimationState | null) {
  return
}

function resolveNumericStats(
  variable: VariableEntry,
  visiblePoints: Array<{ timestampMs: number; value: number }>,
  allPoints: Array<{ timestampMs: number; value: number }>,
  period: PeriodStats,
): NumericStats {
  const analysis = variable.analysis
  const statSource = visiblePoints.length > 0 ? visiblePoints : allPoints
  const minValue = fallbackStat(statSource, 'min')
  const maxValue = fallbackStat(statSource, 'max')
  const meanValue = period.meanValue ?? analysis?.meanValue ?? fallbackStat(statSource, 'mean')
  const medianValue = period.medianValue ?? analysis?.medianValue ?? fallbackStat(statSource, 'median')

  return {
    minValue,
    maxValue,
    meanValue,
    medianValue,
    changeRate: resolveVisibleChangeRate(statSource, analysis),
  }
}

function resolveVisibleChangeRate(
  points: Array<{ timestampMs: number; value: number }>,
  analysis?: UiAnalysisPayload | null,
) {
  const trendPoints = points.slice(-SLOPE_REGRESSION_POINT_COUNT)

  if (trendPoints.length >= 3) {
    const originTimestampMs = trendPoints[0].timestampMs
    let sumX = 0
    let sumY = 0
    let sumXY = 0
    let sumXX = 0

    for (const point of trendPoints) {
      const x = (point.timestampMs - originTimestampMs) / 1000
      const y = point.value
      sumX += x
      sumY += y
      sumXY += x * y
      sumXX += x * x
    }

    const count = trendPoints.length
    const denominator = count * sumXX - sumX * sumX
    if (Number.isFinite(denominator) && Math.abs(denominator) > 1e-6) {
      return (count * sumXY - sumX * sumY) / denominator
    }
  }

  if (trendPoints.length >= 2) {
    const previousPoint = trendPoints[trendPoints.length - 2]
    const latestPoint = trendPoints[trendPoints.length - 1]
    const deltaSeconds = (latestPoint.timestampMs - previousPoint.timestampMs) / 1000
    if (Number.isFinite(deltaSeconds) && Math.abs(deltaSeconds) > 1e-6) {
      return (latestPoint.value - previousPoint.value) / deltaSeconds
    }
  }

  return analysis?.changeRate ?? null
}

function computePeriodStats(points: Array<{ timestampMs: number; value: number }>, analysis?: UiAnalysisPayload | null): PeriodStats {
  const cycle = computeStableCycleStats(points, analysis)
  return {
    periodMs: cycle.periodMs,
    cycleStartsMs: cycle.cycleStartsMs,
    meanValue: cycle.meanValue,
    medianValue: cycle.medianValue,
  }
}

function fallbackStat(points: Array<{ value: number }>, mode: 'min' | 'max' | 'mean' | 'median') {
  if (points.length === 0) {
    return 0
  }

  if (mode === 'mean') {
    return points.reduce((sum, point) => sum + point.value, 0) / points.length
  }

  if (mode === 'median') {
    const values = points.map((point) => point.value).sort((left, right) => left - right)
    const center = Math.floor(values.length / 2)
    if (values.length % 2 === 0) {
      return (values[center - 1] + values[center]) / 2
    }
    return values[center]
  }

  return points.reduce((current, point) => (mode === 'min' ? Math.min(current, point.value) : Math.max(current, point.value)), points[0].value)
}

function formatDisplayValue(variable: VariableEntry, value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return variable.currentValue
  }

  const unit = variable.unit ? normalizeUnitLabel(variable.unit) : ''
  const abs = Math.abs(value)
  const digits = abs >= 100 ? 1 : abs >= 10 ? 2 : 3
  const text = Number.isInteger(value) ? `${value}` : value.toFixed(digits)
  return `${text}${unit}`
}

function createMeasurementOverlayPlugin(modelRef: MutableRefObject<PlotModel | null>): uPlot.Plugin {
  return {
    hooks: {
      init: [
        (u) => {
          const overlayRoot = document.createElement('div')
          overlayRoot.className = 'waveform-overlay'

          u.root.querySelectorAll<HTMLElement>('.u-cursor-pt').forEach((element) => {
            element.style.display = 'none'
          })

          const linesLayer = document.createElementNS(SVG_NS, 'svg')
          linesLayer.classList.add('waveform-overlay-lines')
          linesLayer.setAttribute('aria-hidden', 'true')

          const labelsLayer = document.createElement('div')
          labelsLayer.className = 'waveform-overlay-labels'

          const textTrackLayer = document.createElement('div')
          textTrackLayer.className = 'waveform-text-track'

          overlayRoot.append(linesLayer, labelsLayer, textTrackLayer)
          u.over.appendChild(overlayRoot)

          const state: OverlayState = {
            plot: u,
            overlayRoot,
            linesLayer,
            labelsLayer,
            textTrackLayer,
            items: new Map<string, OverlayItemElements>(),
            cursorAnimations: new Map<string, CursorAnimationState>(),
            cursorFrameId: null,
            latestAnimations: new Map<string, LatestAnimationState>(),
            latestFrameId: null,
            periodLines: new Map<string, SVGLineElement[]>(),
            textTrackSignature: '',
            debugLogIntervalId: null,
          }

          if (isOverlayMotionDebugEnabled()) {
            state.debugLogIntervalId = window.setInterval(() => {
              logOverlayMotionDebug(state)
            }, 500)
          }

            ; (u as uPlot & { __waveformOverlayState?: typeof state }).__waveformOverlayState = state
          syncOverlayItems(u)
        },
      ],
      setSize: [syncOverlaySize, syncOverlayItems, syncOverlayCursor],
      draw: [syncOverlayItems, syncOverlayCursor],
      setCursor: [syncOverlayCursor],
      destroy: [
        (u) => {
          const state = getOverlayState(u)
          if (state && state.cursorFrameId !== null) {
            cancelAnimationFrame(state.cursorFrameId)
          }
          if (state && state.latestFrameId !== null) {
            cancelAnimationFrame(state.latestFrameId)
          }
          if (state && state.debugLogIntervalId !== null) {
            window.clearInterval(state.debugLogIntervalId)
          }
          state?.overlayRoot.remove()
          delete (u as uPlot & { __waveformOverlayState?: unknown }).__waveformOverlayState
        },
      ],
    },
  }

  function syncOverlaySize(u: uPlot) {
    const state = getOverlayState(u)
    if (!state) {
      return null
    }

    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))
    setSvgAttributeIfChanged(state.linesLayer, 'viewBox', `0 0 ${width} ${height}`)
    setSvgAttributeIfChanged(state.linesLayer, 'width', `${width}`)
    setSvgAttributeIfChanged(state.linesLayer, 'height', `${height}`)
    setSvgAttributeIfChanged(state.linesLayer, 'preserveAspectRatio', 'none')
    return { width, height }
  }

  function syncOverlayItems(u: uPlot) {
    const state = getOverlayState(u)
    const model = modelRef.current
    if (!state || !model) {
      return
    }

    const overlaySize = syncOverlaySize(u)
    const width = overlaySize?.width ?? Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = overlaySize?.height ?? Math.max(0, Math.floor(u.over.getBoundingClientRect().height))

    const activeNumericTracks = model.numericTracks.filter(
      (track) => track.labelsEnabled || track.rangeEnabled || track.meanEnabled || track.medianEnabled || track.periodEnabled || track.slopeEnabled,
    )
    const activeTextTracks = model.textTracks.filter((track) => track.textEnabled)
    const latestLabelsEnabled = model.numericTracks.length <= MAX_PERSISTENT_LABEL_TRACKS
    const textTrackSignature = activeTextTracks.map((track) => `${track.name}\u0000${track.value}\u0000${track.color}`).join('|')

    const measurementSlotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []
    for (const track of activeNumericTracks) {
      if (track.meanEnabled) {
        measurementSlotInputs.push({ key: measurementSlotKey(track.name, 'mean'), baseY: safePos(u, track.stats.meanValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
      }
      if (track.medianEnabled) {
        measurementSlotInputs.push({ key: measurementSlotKey(track.name, 'median'), baseY: safePos(u, track.stats.medianValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
      }
    }
    const measurementSlots = placeVerticalLabels(measurementSlotInputs)

    const requiredKeys = new Set<string>()

    for (const track of activeNumericTracks) {
      requiredKeys.add(track.name)
      const elements = getOrCreateOverlayElements(state, track.name)
      const latestPoint = track.points[track.points.length - 1]
      const meanValue = track.stats.meanValue
      const medianValue = track.stats.medianValue
      const minValue = track.stats.minValue
      const maxValue = track.stats.maxValue

      const meanY = safePos(u, meanValue, 'y', height)
      const medianY = safePos(u, medianValue, 'y', height)
      const minY = safePos(u, minValue, 'y', height)
      const maxY = safePos(u, maxValue, 'y', height)

    if (track.rangeEnabled) {
      setLine(elements.minLine, 0, minY, width, minY, colorToRgba(track.color, 0.52), '5 6')
      setLine(elements.maxLine, 0, maxY, width, maxY, colorToRgba(track.color, 0.52), '5 6')
    } else {
      setSvgVisibilityIfChanged(elements.minLine, 'hidden')
      setSvgVisibilityIfChanged(elements.maxLine, 'hidden')
    }

      if (track.periodEnabled && track.period.periodMs !== null) {
        syncPeriodLines(state, u, track, width, minY, maxY, model.windowStartMs, model.windowEndMs)
      } else {
        hidePeriodLines(state.periodLines.get(track.name))
      }

      if (track.meanEnabled) {
        setLine(elements.meanLine, 0, meanY, width, meanY, colorToRgba(track.color, 0.64), '10 6', 1.8)
        updateMeasurementLabel(
          elements.meanLabel,
          width,
          measurementSlots.get(measurementSlotKey(track.name, 'mean')) ?? meanY,
          track.color,
          track.name,
          'AVG',
          formatDisplayValue(track.variable, meanValue),
          'mean',
        )
    } else {
      setSvgVisibilityIfChanged(elements.meanLine, 'hidden')
      setStyleIfChanged(elements.meanLabel.style, 'display', 'none')
    }

      if (track.medianEnabled) {
        setLine(elements.medianLine, 0, medianY, width, medianY, colorToRgba(track.color, 0.54), '3 5', 1.8)
        updateMeasurementLabel(
          elements.medianLabel,
          width,
          measurementSlots.get(measurementSlotKey(track.name, 'median')) ?? medianY,
          track.color,
          track.name,
          'MED',
          formatDisplayValue(track.variable, medianValue),
          'median',
        )
    } else {
      setSvgVisibilityIfChanged(elements.medianLine, 'hidden')
      setStyleIfChanged(elements.medianLabel.style, 'display', 'none')
    }

      setStyleIfChanged(elements.leftLabel.style, 'display', 'none')
      setSvgVisibilityIfChanged(elements.calloutLine, 'hidden')

      if (latestPoint) {
        const latestX = safePos(u, (latestPoint.timestampMs - model.windowStartMs) / 1000, 'x', width)
        const latestY = safePos(u, latestPoint.value, 'y', height)
        const latestLabelX = latestX > width - 170 ? Math.max(12, latestX - 150) : Math.min(width - 150, latestX + 12)
        const latestLabelY = latestY

        if (track.labelsEnabled && latestLabelsEnabled) {
          setStyleIfChanged(elements.latestLabel.style, 'display', 'flex')
          setStyleIfChanged(elements.latestLabel.style, 'borderColor', colorToRgba(track.color, 0.4))
          setStyleIfChanged(elements.latestLabel.style, 'background', colorToRgba(track.color, 0.14))
          setStyleIfChanged(elements.latestLabel.style, 'color', '#f3f7fc')
          setOverlayLabelContent(elements.latestLabel, track.color, track.name, track.displayValue)
          setStyleIfChanged(elements.latestLabel.style, 'left', `${latestLabelX}px`)
          setStyleIfChanged(elements.latestLabel.style, 'top', `${latestLabelY}px`)
          setStyleIfChanged(elements.latestLabel.style, 'transform', 'translateY(-50%)')
          setCircle(elements.latestDot, latestX, latestY, 3.5, track.color)
        } else {
          setCircleRadiusIfChanged(elements.latestDot, 0)
          setStyleIfChanged(elements.latestLabel.style, 'display', 'none')
        }
      } else {
        setCircleRadiusIfChanged(elements.latestDot, 0)
        setStyleIfChanged(elements.latestLabel.style, 'display', 'none')
      }

      if (track.slopeEnabled && track.stats.changeRate !== null && track.stats.changeRate !== undefined && latestPoint) {
        const latestX = (latestPoint.timestampMs - model.windowStartMs) / 1000
        const latestY = latestPoint.value
        const clipped = clipSlopeSegment(
          latestX,
          latestY,
          track.stats.changeRate,
          0,
          minValue,
          maxValue,
        )
        if (clipped) {
          setLine(elements.slopeLine, safePos(u, clipped.startX, 'x', width), safePos(u, clipped.startY, 'y', height), safePos(u, clipped.endX, 'x', width), safePos(u, clipped.endY, 'y', height), colorToRgba(track.color, 0.82), '4 4', 2.2)
        } else {
          setSvgVisibilityIfChanged(elements.slopeLine, 'hidden')
        }
      } else {
        setSvgVisibilityIfChanged(elements.slopeLine, 'hidden')
      }
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        setStyleIfChanged(element.leftLabel.style, 'display', 'none')
        setStyleIfChanged(element.latestLabel.style, 'display', 'none')
        setCircleRadiusIfChanged(element.latestDot, 0)
        setSvgVisibilityIfChanged(element.calloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setStyleIfChanged(element.meanLabel.style, 'display', 'none')
        setStyleIfChanged(element.medianLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.meanLine, 'hidden')
        setSvgVisibilityIfChanged(element.medianLine, 'hidden')
        setSvgVisibilityIfChanged(element.minLine, 'hidden')
        setSvgVisibilityIfChanged(element.maxLine, 'hidden')
        setSvgVisibilityIfChanged(element.slopeLine, 'hidden')
        hidePeriodLines(state.periodLines.get(name))
        state.latestAnimations.delete(name)
      }
    }

    if (state.textTrackSignature !== textTrackSignature) {
      state.textTrackSignature = textTrackSignature
      state.textTrackLayer.replaceChildren()
      for (const track of activeTextTracks) {
        const row = document.createElement('div')
        row.className = 'waveform-text-track-item'
        row.style.borderColor = colorToRgba(track.color, 0.35)
        row.style.background = colorToRgba(track.color, 0.12)
        row.innerHTML = `
          <span class="variable-color" style="background:${track.color}"></span>
          <div class="waveform-text-track-copy">
            <strong>${escapeHtml(track.name)}</strong>
            <small title="${escapeHtml(track.value)}">${escapeHtml(track.value)}</small>
          </div>
        `
        state.textTrackLayer.appendChild(row)
      }
    }
  }

  function syncOverlayCursor(u: uPlot) {
    const state = getOverlayState(u)
    const model = modelRef.current
    if (!state || !model) {
      return
    }

    const { left, top, idx } = u.cursor
    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))

    if (left == null || top == null || left < 0 || top < 0 || idx === null) {
      stopCursorAnimation(state)
      for (const element of state.items.values()) {
        setStyleIfChanged(element.cursorLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.cursorGuideLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorDot, 'hidden')
      }
      return
    }

    const activeNumericTracks = model.numericTracks.filter((track) => track.labelsEnabled || track.slopeEnabled)
    const cursorAnchors = activeNumericTracks.map((track) => {
      const anchor = resolveCursorAnchor(u, track, model.windowStartMs, width)
      const anchorY = safePos(u, anchor.value, 'y', height)
      return {
        track,
        anchor,
        distanceToCursor: Math.abs(anchorY - top),
      }
    })
    const visibleCursorAnchors =
      cursorAnchors.length > MAX_CURSOR_LABEL_TRACKS
        ? [...cursorAnchors].sort((left, right) => left.distanceToCursor - right.distanceToCursor).slice(0, MAX_CURSOR_LABEL_TRACKS)
        : cursorAnchors
    const requiredKeys = new Set<string>()
    for (const { track, anchor: cursorAnchor } of visibleCursorAnchors) {
      requiredKeys.add(track.name)
      const element = getOrCreateOverlayElements(state, track.name)
      const cursorLabelAnchorX = cursorAnchor.x + 14
      const cursorX = clamp(cursorLabelAnchorX, 12, Math.max(12, width - CURSOR_LABEL_MAX_WIDTH_PX - 12))

      setStyleIfChanged(element.cursorLabel.style, 'display', 'flex')
      setStyleIfChanged(element.cursorLabel.style, 'borderColor', colorToRgba(track.color, 0.42))
      setStyleIfChanged(element.cursorLabel.style, 'background', colorToRgba(track.color, 0.16))
      setStyleIfChanged(element.cursorLabel.style, 'color', '#f6f8fb')
      setOverlayLabelContent(element.cursorLabel, track.color, track.name, formatDisplayValue(track.variable, cursorAnchor.value))
      setLine(element.cursorGuideLine, cursorAnchor.x, 0, cursorAnchor.x, height, colorToRgba(track.color, 0.34), '5 5', 1)
      updateCursorAnimationTarget(state, track.name, {
        color: track.color,
        anchorX: cursorAnchor.x,
        labelX: cursorX,
        value: cursorAnchor.value,
      })
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        state.cursorAnimations.delete(name)
        state.latestAnimations.delete(name)
        setStyleIfChanged(element.cursorLabel.style, 'display', 'none')
        setSvgVisibilityIfChanged(element.cursorGuideLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorCalloutLine, 'hidden')
        setSvgVisibilityIfChanged(element.cursorDot, 'hidden')
        setStyleIfChanged(element.latestLabel.style, 'display', 'none')
        setCircleRadiusIfChanged(element.latestDot, 0)
      }
    }

    ensureCursorAnimationFrame(state)
  }

  function getOverlayState(u: uPlot) {
    return (u as uPlot & { __waveformOverlayState?: OverlayState }).__waveformOverlayState ?? null
  }

  function syncPeriodLines(
    state: OverlayState,
    u: uPlot,
    track: PlotTrack,
    width: number,
    minY: number,
    maxY: number,
    windowStartMs: number,
    windowEndMs: number,
  ) {
    const lines = state.periodLines.get(track.name) ?? []
    const cycleStarts = track.period.cycleStartsMs
      .filter((timestampMs) => timestampMs >= windowStartMs && timestampMs <= windowEndMs)
      .sort((left, right) => left - right)

    for (let index = 0; index < cycleStarts.length; index += 1) {
      const timestampMs = cycleStarts[index]!
      const line = lines[index] ?? createSvgLine(state.linesLayer, 'waveform-overlay-period')
      lines[index] = line
      const x = safePos(u, (timestampMs - windowStartMs) / 1000, 'x', width)
      setLine(line, x, minY, x, maxY, colorToRgba(track.color, 0.42), '4 6', 1.4)
    }

    for (let index = cycleStarts.length; index < lines.length; index += 1) {
      lines[index]?.setAttribute('visibility', 'hidden')
    }

    state.periodLines.set(track.name, lines)
  }

  function hidePeriodLines(lines?: SVGLineElement[]) {
    if (!lines) {
      return
    }

    for (const line of lines) {
      line.setAttribute('visibility', 'hidden')
    }
  }

  function getOrCreateOverlayElements(state: OverlayState, name: string) {
    const existing = state.items.get(name)
    if (existing) {
      return existing
    }

    const leftLabel = document.createElement('div')
    leftLabel.className = 'waveform-overlay-label waveform-overlay-label--left'

    const latestLabel = document.createElement('div')
    latestLabel.className = 'waveform-overlay-label waveform-overlay-label--latest'

    const cursorLabel = document.createElement('div')
    cursorLabel.className = 'waveform-overlay-label waveform-overlay-label--cursor'

    const meanLabel = document.createElement('div')
    meanLabel.className = 'waveform-overlay-label waveform-overlay-label--measurement'

    const medianLabel = document.createElement('div')
    medianLabel.className = 'waveform-overlay-label waveform-overlay-label--measurement'

    const calloutLine = createSvgLine(state.linesLayer, 'waveform-overlay-callout')
    const cursorGuideLine = createSvgLine(state.linesLayer, 'waveform-overlay-cursor-guide')
    const cursorCalloutLine = createSvgLine(state.linesLayer, 'waveform-overlay-callout')
    const meanLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const medianLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const minLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const maxLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const slopeLine = createSvgLine(state.linesLayer, 'waveform-overlay-slope')
    const latestDot = createSvgCircle(state.linesLayer, 'waveform-overlay-latest-dot')
    const cursorDot = createSvgCircle(state.linesLayer, 'waveform-overlay-cursor-dot')

    const elements: OverlayItemElements = {
      leftLabel,
      latestLabel,
      cursorLabel,
      meanLabel,
      medianLabel,
      calloutLine,
      cursorGuideLine,
      cursorCalloutLine,
      meanLine,
      medianLine,
      minLine,
      maxLine,
      slopeLine,
      latestDot,
      cursorDot,
    }

    state.labelsLayer.append(leftLabel, latestLabel, cursorLabel, meanLabel, medianLabel)
    state.items.set(name, elements)
    return elements
  }

  function updateCursorAnimationTarget(state: OverlayState, name: string, next: OverlayAnimationTarget) {
    const current = state.cursorAnimations.get(name)
    if (!current) {
      state.cursorAnimations.set(name, {
        currentX: next.labelX,
        currentValue: next.value,
        target: next,
      })
      return
    }

    current.target = next
    if (Math.abs(current.currentX - next.labelX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentX = next.labelX
    }
    if (Math.abs(current.currentValue - next.value) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentValue = next.value
    }
  }

  function updateLatestAnimationTarget(state: OverlayState, name: string, next: OverlayAnimationTarget) {
    const current = state.latestAnimations.get(name)
    if (!current) {
      state.latestAnimations.set(name, {
        currentAnchorX: next.anchorX,
        currentX: next.labelX,
        currentLabelValue: next.value,
        target: next,
      })
      return
    }

    current.target = next
    if (Math.abs(current.currentAnchorX - next.anchorX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentAnchorX = next.anchorX
    }
    if (Math.abs(current.currentX - next.labelX) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentX = next.labelX
    }
    if (Math.abs(current.currentLabelValue - next.value) > CURSOR_SNAP_DISTANCE_PX) {
      current.currentLabelValue = next.value
    }
  }

  function ensureCursorAnimationFrame(state: OverlayState) {
    if (state.cursorFrameId !== null) {
      return
    }

    const tick = () => {
      state.cursorFrameId = null
      let needsNextFrame = false
      const height = Math.max(0, Math.floor(state.plot.over.getBoundingClientRect().height))
      const slotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []

      for (const [name, animation] of state.cursorAnimations) {
        const baseY = safePos(state.plot, animation.currentValue, 'y', height)
        slotInputs.push({
          key: name,
          baseY,
          minY: CURSOR_LABEL_MIN_CENTER_Y,
          maxY: Math.max(CURSOR_LABEL_MIN_CENTER_Y, height - CURSOR_LABEL_MIN_CENTER_Y),
        })
      }

      const cursorSlots = placeVerticalLabels(slotInputs, CURSOR_LABEL_GAP_PX)

      for (const [name, animation] of state.cursorAnimations) {
        const element = state.items.get(name)
        if (!element) {
          continue
        }

        const nextX = stepCursorValue(animation.currentX, animation.target.labelX, CURSOR_SMOOTHING_X)
        const nextValue = stepCursorValue(animation.currentValue, animation.target.value, CURSOR_SMOOTHING_Y)
        const settledX = Math.abs(nextX - animation.target.labelX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledValue = Math.abs(nextValue - animation.target.value) <= CURSOR_ANIMATION_EPSILON_PX

        animation.currentX = settledX ? animation.target.labelX : nextX
        animation.currentValue = settledValue ? animation.target.value : nextValue

        renderCursorAnimationFrame(state.plot, element, animation, cursorSlots.get(name))

        if (!settledX || !settledValue) {
          needsNextFrame = true
        }
      }

      if (needsNextFrame) {
        ensureCursorAnimationFrame(state)
      }
    }

    state.cursorFrameId = requestAnimationFrame(tick)
  }

  function ensureLatestAnimationFrame(state: OverlayState) {
    if (state.latestFrameId !== null) {
      return
    }

    const tick = () => {
      state.latestFrameId = null
      let needsNextFrame = false
      const height = Math.max(0, Math.floor(state.plot.over.getBoundingClientRect().height))
      const slotInputs: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []

      for (const [name, animation] of state.latestAnimations) {
        const baseY = safePos(state.plot, animation.currentLabelValue, 'y', height)
        slotInputs.push({
          key: name,
          baseY,
          minY: 10,
          maxY: Math.max(10, height - 22),
        })
      }

      const latestSlots = placeVerticalLabels(slotInputs)

      for (const [name, animation] of state.latestAnimations) {
        const element = state.items.get(name)
        if (!element) {
          continue
        }

        const nextAnchorX = stepCursorValue(animation.currentAnchorX, animation.target.anchorX, LATEST_SMOOTHING_X)
        const nextX = stepCursorValue(animation.currentX, animation.target.labelX, LATEST_SMOOTHING_X)
        const nextLabelValue = stepCursorValue(animation.currentLabelValue, animation.target.value, LATEST_SMOOTHING_Y)
        const settledAnchorX = Math.abs(nextAnchorX - animation.target.anchorX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledX = Math.abs(nextX - animation.target.labelX) <= CURSOR_ANIMATION_EPSILON_PX
        const settledLabelValue = Math.abs(nextLabelValue - animation.target.value) <= CURSOR_ANIMATION_EPSILON_PX

        animation.currentAnchorX = settledAnchorX ? animation.target.anchorX : nextAnchorX
        animation.currentX = settledX ? animation.target.labelX : nextX
        animation.currentLabelValue = settledLabelValue ? animation.target.value : nextLabelValue

        renderLatestAnimationFrame(state.plot, element, animation, latestSlots.get(name))

        if (!settledAnchorX || !settledX || !settledLabelValue) {
          needsNextFrame = true
        }
      }

      if (needsNextFrame) {
        ensureLatestAnimationFrame(state)
      }
    }

    state.latestFrameId = requestAnimationFrame(tick)
  }

  function stopCursorAnimation(state: OverlayState) {
    if (state.cursorFrameId !== null) {
      cancelAnimationFrame(state.cursorFrameId)
      state.cursorFrameId = null
    }
  }

  function stopLatestAnimation(state: OverlayState) {
    if (state.latestFrameId !== null) {
      cancelAnimationFrame(state.latestFrameId)
      state.latestFrameId = null
    }
  }

  function renderCursorAnimationFrame(plot: uPlot, element: OverlayItemElements, animation: CursorAnimationState, slotY?: number) {
    const { target } = animation
    const height = Math.max(0, Math.floor(plot.over.getBoundingClientRect().height))
    const anchorY = safePos(plot, target.value, 'y', height)
    const labelY = slotY ?? safePos(plot, target.value, 'y', height)
    const shouldShowCallout =
      Math.abs(labelY - anchorY) > CURSOR_CALLOUT_THRESHOLD_PX ||
      Math.abs(animation.currentX - (target.anchorX + 14)) > CURSOR_CALLOUT_THRESHOLD_PX

    setStyleIfChanged(element.cursorLabel.style, 'left', `${animation.currentX}px`)
    setStyleIfChanged(element.cursorLabel.style, 'top', `${labelY}px`)
    setStyleIfChanged(element.cursorLabel.style, 'transform', 'translateY(-50%)')
    setLine(element.cursorGuideLine, target.anchorX, 0, target.anchorX, height, colorToRgba(target.color, 0.34), '5 5', 1)
    setCircle(element.cursorDot, target.anchorX, anchorY, 4.5, target.color)

    if (shouldShowCallout) {
      setLine(element.cursorCalloutLine, target.anchorX, anchorY, animation.currentX - 8, labelY, colorToRgba(target.color, 0.82), '4 4', 1.8)
    } else {
      element.cursorCalloutLine.setAttribute('visibility', 'hidden')
    }
  }

  function renderLatestAnimationFrame(plot: uPlot, element: OverlayItemElements, animation: LatestAnimationState, slotY?: number) {
    const height = Math.max(0, Math.floor(plot.over.getBoundingClientRect().height))
    const anchorY = safePos(plot, animation.target.value, 'y', height)
    const labelY = slotY ?? safePos(plot, animation.currentLabelValue, 'y', height)
    setSvgAttributeIfChanged(element.latestDot, 'cx', `${animation.currentAnchorX}`)
    setSvgAttributeIfChanged(element.latestDot, 'cy', `${anchorY}`)
    setCircleRadiusIfChanged(element.latestDot, 3.5)
    setSvgAttributeIfChanged(element.latestDot, 'fill', animation.target.color)
    setSvgVisibilityIfChanged(element.latestDot, 'visible')

    setStyleIfChanged(element.latestLabel.style, 'left', `${animation.currentX}px`)
    setStyleIfChanged(element.latestLabel.style, 'top', `${labelY}px`)
    setStyleIfChanged(element.latestLabel.style, 'transform', 'translateY(-50%)')
  }

  function stepCursorValue(current: number, target: number, smoothing: number) {
    return current + (target - current) * smoothing
  }

  function logOverlayMotionDebug(state: OverlayState) {
    const latestEntry = state.latestAnimations.entries().next().value as [string, LatestAnimationState] | undefined
    const cursorEntry = state.cursorAnimations.entries().next().value as [string, CursorAnimationState] | undefined

    if (!latestEntry && !cursorEntry) {
      return
    }

    const payload: Record<string, unknown> = {}

    if (latestEntry) {
      const [name, animation] = latestEntry
      payload.latest = {
        name,
        currentAnchorX: roundDebug(animation.currentAnchorX),
        targetAnchorX: roundDebug(animation.target.anchorX),
        dAnchorX: roundDebug(animation.target.anchorX - animation.currentAnchorX),
        currentX: roundDebug(animation.currentX),
        targetX: roundDebug(animation.target.labelX),
        dx: roundDebug(animation.target.labelX - animation.currentX),
        currentValue: roundDebug(animation.currentLabelValue),
        targetValue: roundDebug(animation.target.value),
        dValue: roundDebug(animation.target.value - animation.currentLabelValue),
      }
    }

    if (cursorEntry) {
      const [name, animation] = cursorEntry
      payload.cursor = {
        name,
        currentX: roundDebug(animation.currentX),
        targetX: roundDebug(animation.target.labelX),
        dx: roundDebug(animation.target.labelX - animation.currentX),
        currentValue: roundDebug(animation.currentValue),
        targetValue: roundDebug(animation.target.value),
        dValue: roundDebug(animation.target.value - animation.currentValue),
      }
    }

    console.info(`[waveform-overlay-motion] ${formatDebugPayload(payload)}`)
  }

  function roundDebug(value: number) {
    return Math.round(value * 100) / 100
  }

  function formatDebugPayload(payload: Record<string, unknown>) {
    const latest = formatDebugEntry('latest', payload.latest)
    const cursor = formatDebugEntry('cursor', payload.cursor)
    return [latest, cursor].filter(Boolean).join(' | ')
  }

  function formatDebugEntry(label: string, value: unknown) {
    if (!value || typeof value !== 'object') {
      return ''
    }

    const entry = value as Record<string, unknown>
    return `${label}(name=${entry.name}, currentAnchorX=${entry.currentAnchorX ?? '-'}, targetAnchorX=${entry.targetAnchorX ?? '-'}, dAnchorX=${entry.dAnchorX ?? '-'}, currentX=${entry.currentX}, targetX=${entry.targetX}, dx=${entry.dx}, currentValue=${entry.currentValue}, targetValue=${entry.targetValue}, dValue=${entry.dValue})`
  }

  function setLine(line: SVGLineElement, x1: number, y1: number, x2: number, y2: number, stroke: string, dash?: string, width = 1.6) {
    line.setAttribute('x1', `${x1}`)
    line.setAttribute('y1', `${y1}`)
    line.setAttribute('x2', `${x2}`)
    line.setAttribute('y2', `${y2}`)
    line.setAttribute('stroke', stroke)
    line.setAttribute('stroke-width', `${width}`)
    line.setAttribute('stroke-linecap', 'round')
    line.setAttribute('visibility', 'visible')
    if (dash) {
      line.setAttribute('stroke-dasharray', dash)
    } else {
      line.removeAttribute('stroke-dasharray')
    }
  }

  function setCircle(circle: SVGCircleElement, cx: number, cy: number, r: number, color: string) {
    circle.setAttribute('cx', `${cx}`)
    circle.setAttribute('cy', `${cy}`)
    circle.setAttribute('r', `${r}`)
    circle.setAttribute('fill', color)
    circle.setAttribute('visibility', 'visible')
  }

  function safePos(u: uPlot, value: number, axis: 'x' | 'y', fallback: number) {
    if (!Number.isFinite(value)) {
      return fallback / 2
    }

    const pos = u.valToPos(value, axis)
    return Number.isFinite(pos) ? pos : fallback / 2
  }

  function getLatestValue(track: PlotTrack) {
    return track.points[track.points.length - 1]?.value ?? track.variable.numericValue ?? track.stats.meanValue
  }

  function getCursorValue(u: uPlot, track: PlotTrack) {
    const dataValue = u.data[track.seriesIndex]?.[u.cursor.idx ?? 0]
    if (typeof dataValue === 'number' && Number.isFinite(dataValue)) {
      return dataValue
    }
    return getLatestValue(track)
  }

  function resolveCursorAnchor(u: uPlot, track: PlotTrack, windowStartMs: number, width: number) {
    const cursorIndex = u.cursor.idx ?? 0
    const cursorTime = u.data[0]?.[cursorIndex]
    const cursorX = Number.isFinite(cursorTime) ? safePos(u, Number(cursorTime), 'x', width) : u.cursor.left ?? 0

    if (track.slopeEnabled && !track.labelsEnabled) {
      const slopeValue = getSlopeCursorValue(track, cursorTime, windowStartMs)
      if (slopeValue !== null) {
        return {
          value: slopeValue,
          x: cursorX,
        }
      }
    }

    return {
      value: getCursorValue(u, track),
      x: cursorX,
    }
  }

  function getSlopeCursorValue(track: PlotTrack, cursorTimeSeconds: unknown, windowStartMs: number) {
    if (typeof cursorTimeSeconds !== 'number' || !Number.isFinite(cursorTimeSeconds)) {
      return null
    }

    const latestPoint = track.points[track.points.length - 1]
    const rate = track.stats.changeRate
    if (!latestPoint || rate === null || rate === undefined || !Number.isFinite(rate)) {
      return null
    }

    const anchorX = (latestPoint.timestampMs - windowStartMs) / 1000
    const clipped = clipSlopeSegment(anchorX, latestPoint.value, rate, 0, track.stats.minValue, track.stats.maxValue)
    if (!clipped) {
      return null
    }

    if (cursorTimeSeconds < clipped.startX || cursorTimeSeconds > clipped.endX) {
      return null
    }

    return latestPoint.value + rate * (cursorTimeSeconds - anchorX)
  }

  function placeVerticalLabels(items: Array<{ key: string; baseY: number; minY: number; maxY: number }>, gap = 18) {
    const sorted = [...items].sort((left, right) => left.baseY - right.baseY)
    const slots = new Map<string, number>()

    if (sorted.length === 0) {
      return slots
    }

    const positions = sorted.map((item) => clamp(item.baseY, item.minY, item.maxY))

    for (let index = 1; index < sorted.length; index += 1) {
      positions[index] = clamp(Math.max(positions[index], positions[index - 1] + gap), sorted[index].minY, sorted[index].maxY)
    }

    for (let index = sorted.length - 2; index >= 0; index -= 1) {
      positions[index] = clamp(Math.min(positions[index], positions[index + 1] - gap), sorted[index].minY, sorted[index].maxY)
    }

    for (let index = 1; index < sorted.length; index += 1) {
      if (positions[index] - positions[index - 1] < gap) {
        positions[index] = clamp(positions[index - 1] + gap, sorted[index].minY, sorted[index].maxY)
      }
    }

    for (let index = 0; index < sorted.length; index += 1) {
      slots.set(sorted[index].key, positions[index])
    }

    return slots
  }

  function measurementSlotKey(name: string, kind: 'mean' | 'median') {
    return `${name}:${kind}`
  }

  function updateMeasurementLabel(
    label: HTMLDivElement,
    width: number,
    y: number,
    color: string,
    name: string,
    title: string,
    value: string,
    lane: 'mean' | 'median',
  ) {
    const labelWidth = 180
    const gutter = 10
    const rightLaneLeft = Math.max(12, width - labelWidth)
    const leftLaneLeft = Math.max(12, width - labelWidth * 2 - gutter)

    label.style.display = 'flex'
    setStyleIfChanged(label.style, 'left', `${lane === 'mean' ? leftLaneLeft : rightLaneLeft}px`)
    setStyleIfChanged(label.style, 'top', `${y}px`)
    setStyleIfChanged(label.style, 'transform', 'translateY(-50%)')
    setStyleIfChanged(label.style, 'borderColor', colorToRgba(color, 0.36))
    setStyleIfChanged(label.style, 'background', colorToRgba(color, 0.12))
    setStyleIfChanged(label.style, 'color', '#eaf0f7')
    setOverlayLabelContent(label, color, name, `${title} ${value}`)
  }

  function setOverlayLabelContent(label: HTMLDivElement, color: string, name: string, value: string) {
    const signature = `${color}\u0000${name}\u0000${value}`
    if (label.dataset.overlaySignature === signature) {
      return
    }

    label.dataset.overlaySignature = signature
    label.innerHTML = renderOverlayValueLabel(color, name, value)
  }

  function setStyleIfChanged(style: CSSStyleDeclaration, property: string, value: string) {
    const currentValue = (style as CSSStyleDeclaration & Record<string, string>)[property]
    if (currentValue === value) {
      return
    }

    ;(style as CSSStyleDeclaration & Record<string, string>)[property] = value
  }

  function setSvgAttributeIfChanged(element: SVGElement, name: string, value: string) {
    if (element.getAttribute(name) === value) {
      return
    }

    element.setAttribute(name, value)
  }

  function setSvgVisibilityIfChanged(element: SVGElement, value: 'hidden' | 'visible') {
    setSvgAttributeIfChanged(element, 'visibility', value)
  }

  function setCircleRadiusIfChanged(circle: SVGCircleElement, value: number) {
    const radius = `${value}`
    if (circle.getAttribute('r') === radius) {
      return
    }

    circle.setAttribute('r', radius)
  }

  function renderOverlayValueLabel(color: string, name: string, value: string) {
    return `
      <span class="waveform-overlay-label-chip" style="background:${color}"></span>
      <span class="waveform-overlay-label-copy">
        <strong>${escapeHtml(name)}</strong>
        <span class="waveform-overlay-label-text">${escapeHtml(value)}</span>
      </span>
    `
  }

  function clipSlopeSegment(anchorX: number, anchorY: number, rate: number, xMin: number, yMin: number, yMax: number) {
    if (!Number.isFinite(rate)) {
      return null
    }

    if (!Number.isFinite(xMin) || !Number.isFinite(anchorX) || anchorX < xMin || !Number.isFinite(yMin) || !Number.isFinite(yMax) || yMax < yMin) {
      return null
    }

    if (anchorY < yMin || anchorY > yMax) {
      return null
    }

    if (Math.abs(rate) < 1e-8) {
      return {
        startX: xMin,
        startY: anchorY,
        endX: anchorX,
        endY: anchorY,
      }
    }

    const yBoundary = rate > 0 ? yMin : yMax
    const xAtYBoundary = anchorX + (yBoundary - anchorY) / rate
    const xAtLeftBoundary = xMin
    const yAtLeftBoundary = anchorY + rate * (xAtLeftBoundary - anchorX)

    let startX = xAtLeftBoundary
    let startY = yAtLeftBoundary

    if (Number.isFinite(xAtYBoundary) && xAtYBoundary >= xMin && xAtYBoundary <= anchorX) {
      startX = xAtYBoundary
      startY = yBoundary
    } else if (yAtLeftBoundary < yMin || yAtLeftBoundary > yMax) {
      return null
    }

    return {
      startX,
      startY,
      endX: anchorX,
      endY: anchorY,
    }
  }

  function createSvgLine(parent: SVGSVGElement, className: string) {
    const line = document.createElementNS(SVG_NS, 'line')
    line.classList.add(className)
    parent.appendChild(line)
    return line
  }

  function createSvgCircle(parent: SVGSVGElement, className: string) {
    const circle = document.createElementNS(SVG_NS, 'circle')
    circle.classList.add(className)
    parent.appendChild(circle)
    return circle
  }

  function escapeHtml(value: string) {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/\"/g, '&quot;')
      .replace(/'/g, '&#39;')
  }

  function colorToRgba(color: string, alpha: number) {
    if (!color.startsWith('#')) {
      return color
    }

    const hex = color.slice(1)
    const normalized = hex.length === 3 ? hex.split('').map((char) => `${char}${char}`).join('') : hex
    if (normalized.length !== 6) {
      return color
    }

    const value = Number.parseInt(normalized, 16)
    const red = (value >> 16) & 255
    const green = (value >> 8) & 255
    const blue = value & 255
    return `rgba(${red}, ${green}, ${blue}, ${alpha})`
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value))
  }

  function isOverlayMotionDebugEnabled() {
    if (typeof window === 'undefined') {
      return false
    }

    try {
      const value = window.localStorage.getItem(OVERLAY_MOTION_DEBUG_STORAGE_KEY)
      return value === '1' || value === 'true'
    } catch {
      return false
    }
  }
}

function MagnifyPlusIcon() {
  return (
    <svg viewBox="0 0 20 20" className="waveform-zoom-icon" aria-hidden="true">
      <circle cx="8.25" cy="8.25" r="4.75" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <path d="M11.8 11.8 16 16" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.8" />
      <path d="M8.25 6.25v4M6.25 8.25h4" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.8" />
    </svg>
  )
}

function MagnifyMinusIcon() {
  return (
    <svg viewBox="0 0 20 20" className="waveform-zoom-icon" aria-hidden="true">
      <circle cx="8.25" cy="8.25" r="4.75" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <path d="M11.8 11.8 16 16" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.8" />
      <path d="M6.25 8.25h4" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.8" />
    </svg>
  )
}

type OverlayItemElements = {
  leftLabel: HTMLDivElement
  latestLabel: HTMLDivElement
  cursorLabel: HTMLDivElement
  meanLabel: HTMLDivElement
  medianLabel: HTMLDivElement
  calloutLine: SVGLineElement
  cursorGuideLine: SVGLineElement
  cursorCalloutLine: SVGLineElement
  meanLine: SVGLineElement
  medianLine: SVGLineElement
  minLine: SVGLineElement
  maxLine: SVGLineElement
  slopeLine: SVGLineElement
  latestDot: SVGCircleElement
  cursorDot: SVGCircleElement
}

type OverlayState = {
  plot: uPlot
  overlayRoot: HTMLDivElement
  linesLayer: SVGSVGElement
  labelsLayer: HTMLDivElement
  textTrackLayer: HTMLDivElement
  items: Map<string, OverlayItemElements>
  cursorAnimations: Map<string, CursorAnimationState>
  cursorFrameId: number | null
  latestAnimations: Map<string, LatestAnimationState>
  latestFrameId: number | null
  periodLines: Map<string, SVGLineElement[]>
  textTrackSignature: string
  debugLogIntervalId: number | null
}

type OverlayAnimationTarget = {
  color: string
  anchorX: number
  labelX: number
  value: number
}

type CursorAnimationState = {
  currentX: number
  currentValue: number
  target: OverlayAnimationTarget
}

type LatestAnimationState = {
  currentAnchorX: number
  currentX: number
  currentLabelValue: number
  target: OverlayAnimationTarget
}

type YScaleAnimationState = {
  currentMin: number
  currentMax: number
  targetMin: number
  targetMax: number
  frameId: number | null
}

function normalizeUnitLabel(unit: string) {
  const normalized = unit.trim().toLowerCase()
  if (normalized === 'deg' || normalized === 'degree' || normalized === 'degrees' || normalized === '°') {
    return '°'
  }
  if (normalized === 'percent' || normalized === 'pct' || normalized === '%') {
    return '%'
  }
  if (normalized === 'celsius' || normalized === 'degc' || normalized === '°c') {
    return '°C'
  }
  if (normalized === 'fahrenheit' || normalized === 'degf' || normalized === '°f') {
    return '°F'
  }
  return unit
}

function formatTimeWindow(timeWindowMs: number) {
  return formatWaveformWindowMs(timeWindowMs)
}
