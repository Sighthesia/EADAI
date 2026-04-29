import type uPlot from 'uplot'
import { computeStableCycleStats } from '../../lib/waveformPeriod'
import { isWaveformVisualAidEnabled, type WaveformVisualAidState } from '../../lib/waveformVisualAids'
import { createDevTimingLogger } from '../../lib/logger'
import type { SelectedWaveformVariable } from '../../store/selectors/waveformSelectors'
import type { UiAnalysisPayload, VariableEntry } from '../../types'
import type { NumericStats, PeriodStats, PlotModel, PlotTrack, TextTrack } from './types'
import { SLOPE_REGRESSION_POINT_COUNT } from './types'

const profileBuildPlotModel = createDevTimingLogger('WaveformPanel.buildPlotModel', { slowThresholdMs: 8, summaryEvery: 120, summaryIntervalMs: 5_000 })

// ── Plot model construction ─────────────────────────────────────────────────

export function buildPlotModel(selectedVariables: SelectedWaveformVariable[], visualAidState: WaveformVisualAidState, timeWindowMs: number): PlotModel {
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

// ── Data helpers ────────────────────────────────────────────────────────────

export function isNumericVariable(variable: VariableEntry) {
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

// ── Stats helpers ───────────────────────────────────────────────────────────

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

export function formatDisplayValue(variable: VariableEntry, value?: number | null) {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return variable.currentValue
  }

  const unit = variable.unit ? normalizeUnitLabel(variable.unit) : ''
  const abs = Math.abs(value)
  const digits = abs >= 100 ? 1 : abs >= 10 ? 2 : 3
  const text = Number.isInteger(value) ? `${value}` : value.toFixed(digits)
  return `${text}${unit}`
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
