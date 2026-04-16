import type { SamplePoint, UiAnalysisPayload } from '../types'

const MIN_PERIOD_SIGNAL_RANGE = 0.2

export type StableCycleStats = {
  periodMs: number | null
  cycleStartsMs: number[]
  meanValue: number | null
  medianValue: number | null
}

export function computeStableCycleStats(points: SamplePoint[], analysis?: UiAnalysisPayload | null): StableCycleStats {
  if (points.length < 2) {
    return {
      periodMs: resolveFallbackPeriodMs(analysis),
      cycleStartsMs: [],
      meanValue: null,
      medianValue: null,
    }
  }

  const cycleStartsMs = resolveCycleStarts(points)
  const latestCycleValues = resolveLatestCycleValues(points, cycleStartsMs)

  return {
    periodMs: averagePeriodMs(cycleStartsMs) ?? resolveFallbackPeriodMs(analysis),
    cycleStartsMs,
    meanValue: latestCycleValues.length > 0 ? latestCycleValues.reduce((sum, value) => sum + value, 0) / latestCycleValues.length : null,
    medianValue: computeMedian(latestCycleValues),
  }
}

function resolveFallbackPeriodMs(analysis?: UiAnalysisPayload | null) {
  if (analysis?.periodMs !== undefined && analysis.periodMs !== null) {
    return analysis.periodMs
  }

  const frequencyHz = analysis?.frequencyHz
  if (frequencyHz !== undefined && frequencyHz !== null && Number.isFinite(frequencyHz) && Math.abs(frequencyHz) > 1e-6) {
    return 1000 / frequencyHz
  }

  return null
}

function resolveCycleStarts(points: SamplePoint[]) {
  const minValue = points.reduce((current, point) => Math.min(current, point.value), points[0]!.value)
  const maxValue = points.reduce((current, point) => Math.max(current, point.value), points[0]!.value)
  const range = maxValue - minValue
  if (!Number.isFinite(range) || range < MIN_PERIOD_SIGNAL_RANGE) {
    return []
  }

  const midpoint = minValue + range / 2
  const hysteresis = Math.max(range * 0.1, 0.02)
  const upper = midpoint + hysteresis
  const lower = midpoint - hysteresis
  let state = points[0]!.value >= midpoint
  const risingStarts: number[] = []
  const fallingStarts: number[] = []

  for (let index = 1; index < points.length; index += 1) {
    const point = points[index]!
    const nextState = point.value >= upper ? true : point.value <= lower ? false : state
    if (nextState !== state) {
      if (nextState) {
        risingStarts.push(point.timestampMs)
      } else {
        fallingStarts.push(point.timestampMs)
      }
      state = nextState
    }
  }

  if (risingStarts.length >= 2) {
    return risingStarts
  }
  if (fallingStarts.length >= 2) {
    return fallingStarts
  }
  return []
}

function resolveLatestCycleValues(points: SamplePoint[], cycleStartsMs: number[]) {
  if (cycleStartsMs.length < 2) {
    return []
  }

  const startMs = cycleStartsMs[cycleStartsMs.length - 2]!
  const endMs = cycleStartsMs[cycleStartsMs.length - 1]!
  return points.filter((point) => point.timestampMs >= startMs && point.timestampMs < endMs).map((point) => point.value)
}

function averagePeriodMs(cycleStartsMs: number[]) {
  if (cycleStartsMs.length < 2) {
    return null
  }

  let total = 0
  for (let index = 1; index < cycleStartsMs.length; index += 1) {
    total += cycleStartsMs[index]! - cycleStartsMs[index - 1]!
  }
  return total / (cycleStartsMs.length - 1)
}

function computeMedian(values: number[]) {
  if (values.length === 0) {
    return null
  }

  const sorted = [...values].sort((left, right) => left - right)
  const center = Math.floor(sorted.length / 2)
  return sorted.length % 2 === 0 ? (sorted[center - 1]! + sorted[center]!) / 2 : sorted[center]!
}
