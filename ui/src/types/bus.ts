import type { UiSource, UiConnectionPayload } from './session'
import type { UiLinePayload, UiParserMeta } from './line'
import type { UiTelemetrySchemaPayload, UiTelemetryIdentityPayload, UiTelemetrySamplePayload } from './protocol'
import type { UiAnalysisPayload, UiTriggerPayload } from './analysis'
import type { VariableSourceKind } from './variables'

export type SerialBusEvent =
  | {
      kind: 'connection'
      timestampMs: number
      source: UiSource
      connection: UiConnectionPayload
    }
  | {
      kind: 'line'
      timestampMs: number
      source: UiSource
      line: UiLinePayload
      parser: UiParserMeta
    }
  | {
      kind: 'shellOutput'
      timestampMs: number
      source: UiSource
      line: UiLinePayload
      parser: UiParserMeta
    }
  | {
      kind: 'analysis'
      timestampMs: number
      source: UiSource
      analysis: UiAnalysisPayload
    }
  | {
      kind: 'telemetrySchema'
      timestampMs: number
      source: UiSource
      schema: UiTelemetrySchemaPayload
      rawFrame: number[]
      parser: UiParserMeta
    }
  | {
      kind: 'telemetryIdentity'
      timestampMs: number
      source: UiSource
      identity: UiTelemetryIdentityPayload
      rawFrame: number[]
      parser: UiParserMeta
    }
  | {
      kind: 'telemetrySample'
      timestampMs: number
      source: UiSource
      sample: UiTelemetrySamplePayload
      rawFrame: number[]
      parser: UiParserMeta
    }
  | {
      kind: 'trigger'
      timestampMs: number
      source: UiSource
      trigger: UiTriggerPayload
    }

export interface ConsoleEntry {
  id: string
  direction: 'rx' | 'tx'
  text: string
  raw: number[]
  timestampMs: number
  parser?: UiParserMeta | null
}

export interface SamplePoint {
  timestampMs: number
  value: number
}

export interface VariableEntry {
  name: string
  deviceRef?: string | null
  sourceKind: VariableSourceKind
  currentValue: string
  previousValue?: number
  numericValue?: number
  unit?: string | null
  trend: 'up' | 'down' | 'flat'
  parserName?: string | null
  sampleCount: number
  updatedAtMs: number
  points: SamplePoint[]
  analysis?: UiAnalysisPayload | null
  latestTrigger?: UiTriggerPayload | null
  triggerCount: number
}
