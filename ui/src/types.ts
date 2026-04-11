export type UiConnectionState = 'idle' | 'connecting' | 'connected' | 'waitingRetry' | 'stopped'
export type UiLineDirection = 'rx' | 'tx'
export type UiTransportKind = 'serial' | 'fake'
export type SourceKind = 'serial' | 'fake'

export interface ConnectRequest {
  port: string
  baudRate: number
  retryMs: number
  readTimeoutMs: number
  sourceKind: SourceKind
  fakeProfile?: string | null
}

export interface SendRequest {
  payload: string
  appendNewline: boolean
}

export interface SessionSnapshot {
  isRunning: boolean
  transport?: UiTransportKind | null
  port?: string | null
  baudRate?: number | null
  connectionState?: UiConnectionState | null
}

export interface UiSource {
  transport: UiTransportKind
  port: string
  baudRate: number
}

export interface UiParserMeta {
  parserName?: string | null
  fields: Record<string, string>
}

export interface UiConnectionPayload {
  state: UiConnectionState
  reason?: string | null
  attempt: number
  retryDelayMs?: number | null
}

export interface UiLinePayload {
  direction: UiLineDirection
  text: string
  rawLength: number
}

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

export interface ConsoleEntry {
  id: string
  direction: UiLineDirection
  text: string
  timestampMs: number
}

export interface SamplePoint {
  timestampMs: number
  value: number
}

export interface VariableEntry {
  name: string
  currentValue: string
  previousValue?: number
  numericValue?: number
  trend: 'up' | 'down' | 'flat'
  parserName?: string | null
  sampleCount: number
  updatedAtMs: number
  points: SamplePoint[]
}

export interface AppStatus {
  tone: 'neutral' | 'success' | 'warning' | 'danger'
  message: string
}
