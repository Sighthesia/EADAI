import type {
  AppStatus,
  SamplePoint,
  SerialBusEvent,
  UiAnalysisPayload,
  UiTriggerPayload,
  VariableEntry,
} from '../types'
import { CHANNEL_COLORS, MAX_SAMPLES_PER_CHANNEL } from './constants'

export const applyAnalysis = (
  variable: VariableEntry,
  analysis: UiAnalysisPayload,
  updatedAtMs: number,
): VariableEntry => ({
  ...variable,
  analysis,
  trend: trendFromDelta(analysis.trend),
  updatedAtMs,
})

export const autoSelectChannels = (selectedChannels: string[], channelId: string) => {
  if (selectedChannels.includes(channelId)) {
    return selectedChannels
  }
  if (selectedChannels.length > 0) {
    return selectedChannels
  }
  return [channelId]
}

export const trendFromDelta = (delta: number | null | undefined) => {
  if (delta === undefined || delta === null) {
    return 'flat' as const
  }
  if (delta > 0) {
    return 'up' as const
  }
  if (delta < 0) {
    return 'down' as const
  }
  return 'flat' as const
}

export const resolveTrendFromValues = (previousValue: number | undefined, nextValue: number | undefined) => {
  if (previousValue === undefined || previousValue === null || nextValue === undefined || nextValue === null) {
    return 'flat' as const
  }

  return trendFromDelta(nextValue - previousValue)
}

export const severityTone = (severity: UiTriggerPayload['severity']): AppStatus['tone'] => {
  switch (severity) {
    case 'critical':
      return 'danger'
    case 'warning':
      return 'warning'
    case 'info':
      return 'neutral'
  }
}

export const triggerMessage = (trigger: UiTriggerPayload) =>
  `Trigger ${trigger.ruleId} on ${trigger.channelId}: ${trigger.reason}`

export const parseSampleTimestampMs = (timestamp: string | undefined, fallback: number) => {
  if (!timestamp) {
    return fallback
  }

  const parsed = Number(timestamp)
  return Number.isFinite(parsed) ? parsed : fallback
}

export const colorForChannel = (channel: string) => {
  let hash = 0
  for (const char of channel) {
    hash = (hash << 5) - hash + char.charCodeAt(0)
    hash |= 0
  }
  return CHANNEL_COLORS[Math.abs(hash) % CHANNEL_COLORS.length]
}

export const fakePortLabel = (profile: string) => `fake://${profile}`

export const createVariableEntry = (
  channelId: string,
  updatedAtMs: number,
  deviceRef: string | null,
  sourceKind: 'protocol-text' | 'telemetry-sample' = 'protocol-text',
): VariableEntry => ({
  name: channelId,
  deviceRef,
  sourceKind,
  currentValue: '—',
  trend: 'flat',
  unit: null,
  parserName: null,
  sampleCount: 0,
  updatedAtMs,
  points: [],
  analysis: null,
  latestTrigger: null,
  triggerCount: 0,
})

export const updateVariableFromProtocolText = (
  previous: VariableEntry,
  channelId: string,
  event: Extract<SerialBusEvent, { kind: 'line' }>,
  rawValue: string,
  numericValue: number,
  nextPoints: SamplePoint[],
): VariableEntry => ({
  ...previous,
  name: channelId,
  sourceKind: 'protocol-text',
  currentValue: rawValue,
  previousValue: previous.numericValue,
  numericValue: Number.isFinite(numericValue) ? numericValue : previous.numericValue,
  trend: resolveTrendFromValues(previous.numericValue, Number.isFinite(numericValue) ? numericValue : previous.numericValue),
  unit: event.parser.fields.unit ?? previous.unit ?? null,
  parserName: event.parser.parserName,
  sampleCount: previous.sampleCount + 1,
  updatedAtMs: event.timestampMs,
  points: nextPoints,
})

export const updateVariableFromTelemetrySample = (
  previous: VariableEntry,
  channelId: string,
  event: Extract<SerialBusEvent, { kind: 'telemetrySample' }>,
  nextNumericValue: number,
  displayValue: string,
  unit: string | null,
): VariableEntry => ({
  ...previous,
  name: channelId,
  sourceKind: 'telemetry-sample',
  currentValue: displayValue,
  previousValue: previous.numericValue,
  numericValue: nextNumericValue,
  trend: resolveTrendFromValues(previous.numericValue, nextNumericValue),
  unit,
  parserName: 'bmi088_sample',
  sampleCount: previous.sampleCount + 1,
  updatedAtMs: event.timestampMs,
  points: [...previous.points, { timestampMs: event.timestampMs, value: nextNumericValue }].slice(-MAX_SAMPLES_PER_CHANNEL),
})
