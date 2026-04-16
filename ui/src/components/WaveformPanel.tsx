import { useEffect, useMemo, useRef, useState, type MutableRefObject } from 'react'
import uPlot from 'uplot'
import { isWaveformVisualAidEnabled, type WaveformVisualAidState } from '../lib/waveformVisualAids'
import { useAppStore } from '../store/appStore'
import type { UiAnalysisPayload, VariableEntry } from '../types'

const MIN_TIME_WINDOW_MS = 2_000
const MAX_TIME_WINDOW_MS = 120_000
const DEFAULT_TIME_WINDOW_MS = 15_000
const SLOPE_REGRESSION_POINT_COUNT = 8

const SVG_NS = 'http://www.w3.org/2000/svg'

type SelectedVariable = {
  name: string
  color: string
  variable: VariableEntry
}

type NumericStats = {
  minValue: number
  maxValue: number
  meanValue: number
  medianValue: number
  changeRate?: number | null
}

type PlotTrack = {
  name: string
  color: string
  variable: VariableEntry
  labelsEnabled: boolean
  rangeEnabled: boolean
  meanEnabled: boolean
  medianEnabled: boolean
  slopeEnabled: boolean
  points: Array<{ timestampMs: number; value: number }>
  displayValue: string
  seriesIndex: number
  stats: NumericStats
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
}

export function WaveformPanel() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const visualAidState = useAppStore((state) => state.visualAidState)
  const [timeWindowMs, setTimeWindowMs] = useState(DEFAULT_TIME_WINDOW_MS)
  const [menuOpen, setMenuOpen] = useState(true)

  const selectedVariables = useMemo<SelectedVariable[]>(
    () =>
      selectedChannels
        .map((channel) => variables[channel])
        .filter(Boolean)
        .map((variable) => ({
          name: variable.name,
          color: colorForChannel(variable.name),
          variable,
        })),
    [colorForChannel, selectedChannels, variables],
  )

  const numericCount = selectedVariables.filter(({ variable }) => isNumericVariable(variable)).length
  const textCount = selectedVariables.length - numericCount

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
          <div className="waveform-floating-heading">
            <strong>Waveform Controls</strong>
            <small>{selectedVariables.length > 0 ? 'Pan with mouse wheel; tune the overlay controls here.' : 'Select variables to start plotting.'}</small>
          </div>
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
                    min={MIN_TIME_WINDOW_MS}
                    max={MAX_TIME_WINDOW_MS}
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
  selectedVariables: SelectedVariable[]
  visualAidState: WaveformVisualAidState
  timeWindowMs: number
  onTimeWindowChange: (value: number | ((current: number) => number)) => void
}) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  const plotRef = useRef<uPlot | null>(null)
  const structureKeyRef = useRef('')
  const modelRef = useRef<PlotModel | null>(null)
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
          scales: { x: { time: false } },
          axes: [
            {
              stroke: '#5f6b7a',
              grid: { stroke: '#20242d' },
              values: (_, ticks) => ticks.map((tick) => `${tick.toFixed(tick >= 10 ? 0 : 1)}s`),
            },
            { stroke: '#5f6b7a', grid: { stroke: '#20242d' } },
          ],
          series: model.series,
          plugins: [createMeasurementOverlayPlugin(modelRef)],
        },
        model.data,
        hostRef.current,
      )
      structureKeyRef.current = structureKey
    }

    return () => {
      if (!hostRef.current?.isConnected) {
        plotRef.current?.destroy()
        plotRef.current = null
        structureKeyRef.current = ''
      }
    }
}, [model.data, model.series, size, structureKey])

  useEffect(() => {
    plotRef.current?.setSize(size)
  }, [size])

  useEffect(() => {
    plotRef.current?.setData(model.data)
  }, [model.data])

  useEffect(() => {
    plotRef.current?.redraw()
  }, [visualAidState])

  useEffect(
    () => () => {
      plotRef.current?.destroy()
      plotRef.current = null
      structureKeyRef.current = ''
    },
    [],
  )

  return <div className="wave-plot waveform-stage-surface" ref={hostRef} />
}

function buildPlotModel(selectedVariables: SelectedVariable[], visualAidState: WaveformVisualAidState, timeWindowMs: number): PlotModel {
  const tracks = selectedVariables.map(({ name, color, variable }) => ({
    name,
    color,
    variable,
    labelsEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'labels'),
    rangeEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'range'),
    meanEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'mean'),
    medianEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'median'),
    slopeEnabled: isWaveformVisualAidEnabled(visualAidState, name, 'slope'),
  }))

  const latestPointTimestampMs = tracks.reduce((max, item) => {
    const pointMax = item.variable.points.reduce((latest, point) => Math.max(latest, point.timestampMs), Number.NEGATIVE_INFINITY)
    return Math.max(max, pointMax)
  }, Number.NEGATIVE_INFINITY)

  const latestUpdateTimestampMs = tracks.reduce((max, item) => Math.max(max, item.variable.updatedAtMs), Number.NEGATIVE_INFINITY)

  const latestTimestampMs = Number.isFinite(latestPointTimestampMs)
    ? latestPointTimestampMs
    : Number.isFinite(latestUpdateTimestampMs)
      ? latestUpdateTimestampMs
      : Date.now()

  const windowEndMs = latestTimestampMs
  const windowStartMs = windowEndMs - timeWindowMs

  const numericTracks = tracks
    .filter(({ variable }) => isNumericVariable(variable))
    .map((item, index) => {
      const sourcePoints = item.variable.points.length > 0 ? item.variable.points : createFallbackNumericPoint(item.variable, windowEndMs)
      const visiblePoints = sourcePoints.filter((point) => point.timestampMs >= windowStartMs)
      const stats = resolveNumericStats(item.variable, visiblePoints, sourcePoints)
      return {
        name: item.name,
        color: item.color,
        variable: item.variable,
        labelsEnabled: item.labelsEnabled,
        rangeEnabled: item.rangeEnabled,
        meanEnabled: item.meanEnabled,
        medianEnabled: item.medianEnabled,
        slopeEnabled: item.slopeEnabled,
        points: visiblePoints,
        displayValue: formatDisplayValue(
          item.variable,
          item.variable.numericValue ?? visiblePoints[visiblePoints.length - 1]?.value ?? sourcePoints[sourcePoints.length - 1]?.value,
        ),
        seriesIndex: index + 1,
        stats,
      }
    })

  const textTracks = tracks
    .filter(({ variable }) => !isNumericVariable(variable))
    .map((item) => ({
      name: item.name,
      color: item.color,
      textEnabled: isWaveformVisualAidEnabled(visualAidState, item.name, 'text'),
      value: item.variable.currentValue,
      updatedAtMs: item.variable.updatedAtMs,
    }))

  const timestamps = Array.from(new Set(numericTracks.flatMap((item) => item.points.map((point) => point.timestampMs)))).sort((left, right) => left - right)

  if (numericTracks.length === 0 || timestamps.length === 0) {
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
      windowStartMs,
      windowEndMs,
    }
  }

  const normalizedTimestamps = timestamps.map((timestamp) => (timestamp - windowStartMs) / 1000)
  const showPoints = normalizedTimestamps.length <= 240
  const data: Array<number[] | Array<number | null>> = [normalizedTimestamps]
  const plotSeries = [{ label: 'time' } as uPlot.Series]

  for (const item of numericTracks) {
    const valueByTimestamp = new Map(item.points.map((point) => [point.timestampMs, point.value]))
    data.push(timestamps.map((timestamp) => valueByTimestamp.get(timestamp) ?? null))
    plotSeries.push({
      label: item.name,
      stroke: item.color,
      width: 2,
      points: showPoints ? { show: true, size: 4, width: 1 } : { show: false },
    } as uPlot.Series)
  }

  return {
    data: data as unknown as uPlot.AlignedData,
    series: plotSeries,
    numericTracks,
    textTracks,
    windowStartMs,
    windowEndMs,
  }
}

function scaleTimeWindow(current: number, factor: number) {
  return clampTimeWindow(Math.round(current * factor))
}

function clampTimeWindow(value: number) {
  return Math.min(MAX_TIME_WINDOW_MS, Math.max(MIN_TIME_WINDOW_MS, value))
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

function resolveNumericStats(
  variable: VariableEntry,
  visiblePoints: Array<{ timestampMs: number; value: number }>,
  allPoints: Array<{ timestampMs: number; value: number }>,
): NumericStats {
  const analysis = variable.analysis
  const statSource = visiblePoints.length > 0 ? visiblePoints : allPoints
  const minValue = fallbackStat(statSource, 'min')
  const maxValue = fallbackStat(statSource, 'max')
  const meanValue = analysis?.meanValue ?? fallbackStat(statSource, 'mean')
  const medianValue = analysis?.medianValue ?? fallbackStat(statSource, 'median')

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

          const linesLayer = document.createElementNS(SVG_NS, 'svg')
          linesLayer.classList.add('waveform-overlay-lines')
          linesLayer.setAttribute('aria-hidden', 'true')

          const labelsLayer = document.createElement('div')
          labelsLayer.className = 'waveform-overlay-labels'

          const textTrackLayer = document.createElement('div')
          textTrackLayer.className = 'waveform-text-track'

          overlayRoot.append(linesLayer, labelsLayer, textTrackLayer)
          u.over.appendChild(overlayRoot)

          const state = {
            overlayRoot,
            linesLayer,
            labelsLayer,
            textTrackLayer,
            items: new Map<string, OverlayItemElements>(),
            cursorItems: new Map<string, HTMLDivElement>(),
            textTrackSignature: '',
          }

          ;(u as uPlot & { __waveformOverlayState?: typeof state }).__waveformOverlayState = state
          syncOverlayItems(u)
        },
      ],
      setSize: [syncOverlaySize, syncOverlayItems],
      draw: [syncOverlayItems],
      setCursor: [syncOverlayCursor],
      destroy: [
        (u) => {
          const state = getOverlayState(u)
          state?.overlayRoot.remove()
          delete (u as uPlot & { __waveformOverlayState?: unknown }).__waveformOverlayState
        },
      ],
    },
  }

  function syncOverlaySize(u: uPlot) {
    const state = getOverlayState(u)
    if (!state) {
      return
    }

    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))
    state.linesLayer.setAttribute('viewBox', `0 0 ${width} ${height}`)
    state.linesLayer.setAttribute('width', `${width}`)
    state.linesLayer.setAttribute('height', `${height}`)
    state.linesLayer.setAttribute('preserveAspectRatio', 'none')
  }

  function syncOverlayItems(u: uPlot) {
    const state = getOverlayState(u)
    const model = modelRef.current
    if (!state || !model) {
      return
    }

    syncOverlaySize(u)
    const width = Math.max(0, Math.floor(u.over.getBoundingClientRect().width))
    const height = Math.max(0, Math.floor(u.over.getBoundingClientRect().height))

    const activeNumericTracks = model.numericTracks.filter(
      (track) => track.labelsEnabled || track.rangeEnabled || track.meanEnabled || track.medianEnabled || track.slopeEnabled,
    )
    const activeTextTracks = model.textTracks.filter((track) => track.textEnabled)
    const textTrackSignature = activeTextTracks.map((track) => `${track.name}\u0000${track.value}\u0000${track.color}`).join('|')

    const sortedByMean = [...activeNumericTracks].sort((left, right) => left.stats.meanValue - right.stats.meanValue)
    const latestSlots = placeVerticalLabels(sortedByMean.map((item) => ({ key: item.name, baseY: safePos(u, getLatestValue(item), 'y', height), minY: 10, maxY: Math.max(10, height - 22) })))
    const measurementSlots = placeVerticalLabels(
      activeNumericTracks.flatMap((track) => {
        const slots: Array<{ key: string; baseY: number; minY: number; maxY: number }> = []
        if (track.meanEnabled) {
          slots.push({ key: measurementSlotKey(track.name, 'mean'), baseY: safePos(u, track.stats.meanValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
        }
        if (track.medianEnabled) {
          slots.push({ key: measurementSlotKey(track.name, 'median'), baseY: safePos(u, track.stats.medianValue, 'y', height), minY: 14, maxY: Math.max(14, height - 14) })
        }
        return slots
      }),
    )

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
        elements.minLine.setAttribute('visibility', 'hidden')
        elements.maxLine.setAttribute('visibility', 'hidden')
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
        elements.meanLine.setAttribute('visibility', 'hidden')
        elements.meanLabel.style.display = 'none'
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
        elements.medianLine.setAttribute('visibility', 'hidden')
        elements.medianLabel.style.display = 'none'
      }

      elements.leftLabel.style.display = 'none'
      elements.calloutLine.setAttribute('visibility', 'hidden')

      if (latestPoint) {
        const latestX = safePos(u, (latestPoint.timestampMs - model.windowStartMs) / 1000, 'x', width)
        const latestY = safePos(u, latestPoint.value, 'y', height)
        const latestLabelY = latestSlots.get(track.name) ?? latestY
        const latestLabelX = latestX > width - 170 ? Math.max(12, latestX - 150) : Math.min(width - 150, latestX + 12)

        if (track.labelsEnabled) {
          setCircle(elements.latestDot, latestX, latestY, 3.5, track.color)
          elements.latestLabel.style.display = 'flex'
          elements.latestLabel.style.left = `${latestLabelX}px`
          elements.latestLabel.style.top = `${latestLabelY}px`
          elements.latestLabel.style.transform = 'translateY(-50%)'
          elements.latestLabel.style.borderColor = colorToRgba(track.color, 0.4)
          elements.latestLabel.style.background = colorToRgba(track.color, 0.14)
          elements.latestLabel.style.color = '#f3f7fc'
          elements.latestLabel.innerHTML = renderOverlayValueLabel(track.color, track.name, track.displayValue)
        } else {
          elements.latestDot.setAttribute('r', '0')
          elements.latestLabel.style.display = 'none'
        }
      } else {
        elements.latestDot.setAttribute('r', '0')
        elements.latestLabel.style.display = 'none'
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
          setLine(elements.slopeLine, safePos(u, clipped.startX, 'x', width), safePos(u, clipped.startY, 'y', height), safePos(u, clipped.endX, 'x', width), safePos(u, clipped.endY, 'y', height), colorToRgba(track.color, 0.82), undefined, 2.2)
        } else {
          elements.slopeLine.setAttribute('visibility', 'hidden')
        }
      } else {
        elements.slopeLine.setAttribute('visibility', 'hidden')
      }
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        element.leftLabel.style.display = 'none'
        element.latestLabel.style.display = 'none'
        element.latestDot.setAttribute('r', '0')
        element.calloutLine.setAttribute('visibility', 'hidden')
        element.cursorCalloutLine.setAttribute('visibility', 'hidden')
        element.meanLabel.style.display = 'none'
        element.medianLabel.style.display = 'none'
        element.meanLine.setAttribute('visibility', 'hidden')
        element.medianLine.setAttribute('visibility', 'hidden')
        element.minLine.setAttribute('visibility', 'hidden')
        element.maxLine.setAttribute('visibility', 'hidden')
        element.slopeLine.setAttribute('visibility', 'hidden')
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
      for (const element of state.items.values()) {
        element.cursorLabel.style.display = 'none'
        element.cursorCalloutLine.setAttribute('visibility', 'hidden')
      }
      return
    }

    const activeNumericTracks = model.numericTracks.filter((track) => track.labelsEnabled || track.slopeEnabled)
    const cursorAnchors = activeNumericTracks.map((track) => ({
      track,
      anchor: resolveCursorAnchor(u, track, model.windowStartMs, width),
    }))
    const cursorSlots = placeVerticalLabels(
      cursorAnchors.map(({ track, anchor }) => ({
        key: track.name,
        baseY: safePos(u, anchor.value, 'y', height),
        minY: 10,
        maxY: Math.max(10, height - 22),
      })),
    )

    const requiredKeys = new Set<string>()
    for (const { track, anchor: cursorAnchor } of cursorAnchors) {
      requiredKeys.add(track.name)
      const element = getOrCreateOverlayElements(state, track.name)
      const cursorY = safePos(u, cursorAnchor.value, 'y', height)
      const cursorX = clamp(cursorAnchor.x + 14, 12, Math.max(12, width - 160))
      const cursorYPlacement = cursorSlots.get(track.name) ?? cursorY

      element.cursorLabel.style.display = 'flex'
      element.cursorLabel.style.left = `${cursorX}px`
      element.cursorLabel.style.top = `${cursorYPlacement}px`
      element.cursorLabel.style.transform = 'translateY(-50%)'
      element.cursorLabel.style.borderColor = colorToRgba(track.color, 0.42)
      element.cursorLabel.style.background = colorToRgba(track.color, 0.16)
      element.cursorLabel.style.color = '#f6f8fb'
      element.cursorLabel.innerHTML = renderOverlayValueLabel(track.color, track.name, formatDisplayValue(track.variable, cursorAnchor.value))
      setLine(element.cursorCalloutLine, cursorAnchor.x, cursorY, cursorX - 6, cursorYPlacement, colorToRgba(track.color, 0.76))
    }

    for (const [name, element] of state.items) {
      if (!requiredKeys.has(name)) {
        element.cursorLabel.style.display = 'none'
        element.cursorCalloutLine.setAttribute('visibility', 'hidden')
      }
    }
  }

  function getOverlayState(u: uPlot) {
    return (u as uPlot & { __waveformOverlayState?: OverlayState }).__waveformOverlayState ?? null
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
    const cursorCalloutLine = createSvgLine(state.linesLayer, 'waveform-overlay-callout')
    const meanLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const medianLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const minLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const maxLine = createSvgLine(state.linesLayer, 'waveform-overlay-band')
    const slopeLine = createSvgLine(state.linesLayer, 'waveform-overlay-slope')
    const latestDot = createSvgCircle(state.linesLayer, 'waveform-overlay-latest-dot')

    const elements: OverlayItemElements = {
      leftLabel,
      latestLabel,
      cursorLabel,
      meanLabel,
      medianLabel,
      calloutLine,
      cursorCalloutLine,
      meanLine,
      medianLine,
      minLine,
      maxLine,
      slopeLine,
      latestDot,
    }

    state.labelsLayer.append(leftLabel, latestLabel, cursorLabel, meanLabel, medianLabel)
    state.items.set(name, elements)
    return elements
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

  function placeVerticalLabels(items: Array<{ key: string; baseY: number; minY: number; maxY: number }>) {
    const sorted = [...items].sort((left, right) => left.baseY - right.baseY)
    const slots = new Map<string, number>()

    if (sorted.length === 0) {
      return slots
    }

    const gap = 18
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
    label.style.left = `${lane === 'mean' ? leftLaneLeft : rightLaneLeft}px`
    label.style.top = `${y}px`
    label.style.transform = 'translateY(-50%)'
    label.style.borderColor = colorToRgba(color, 0.36)
    label.style.background = colorToRgba(color, 0.12)
    label.style.color = '#eaf0f7'
    label.innerHTML = renderOverlayValueLabel(color, name, `${title} ${value}`)
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
  cursorCalloutLine: SVGLineElement
  meanLine: SVGLineElement
  medianLine: SVGLineElement
  minLine: SVGLineElement
  maxLine: SVGLineElement
  slopeLine: SVGLineElement
  latestDot: SVGCircleElement
}

type OverlayState = {
  overlayRoot: HTMLDivElement
  linesLayer: SVGSVGElement
  labelsLayer: HTMLDivElement
  textTrackLayer: HTMLDivElement
  items: Map<string, OverlayItemElements>
  cursorItems: Map<string, HTMLDivElement>
  textTrackSignature: string
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
  if (timeWindowMs < 10_000) {
    return `${(timeWindowMs / 1000).toFixed(1)}s`
  }
  if (timeWindowMs < 60_000) {
    return `${Math.round(timeWindowMs / 1000)}s`
  }
  const minutes = timeWindowMs / 60_000
  return `${minutes.toFixed(minutes >= 10 ? 0 : 1)}m`
}
