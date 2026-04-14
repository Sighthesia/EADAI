export type UiConnectionState = 'idle' | 'connecting' | 'connected' | 'waitingRetry' | 'stopped'
export type UiLineDirection = 'rx' | 'tx'
export type UiTransportKind = 'serial' | 'fake'
export type UiTriggerSeverity = 'info' | 'warning' | 'critical'
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

export interface McpServerStatus {
  isRunning: boolean
  transport: string
  endpointUrl?: string | null
  lastError?: string | null
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

export interface UiAnalysisPayload {
  channelId: string
  windowMs: number
  sampleCount: number
  timeSpanMs?: number | null
  frequencyHz?: number | null
  periodMs?: number | null
  periodStability?: number | null
  dutyCycle?: number | null
  minValue?: number | null
  maxValue?: number | null
  meanValue?: number | null
  rmsValue?: number | null
  variance?: number | null
  edgeCount: number
  risingEdgeCount: number
  fallingEdgeCount: number
  trend?: number | null
  changeRate?: number | null
  triggerHits: string[]
}

export interface UiTriggerPayload {
  channelId: string
  ruleId: string
  severity: UiTriggerSeverity
  firedAtMs: number
  reason: string
  snapshot?: UiAnalysisPayload | null
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
  | {
      kind: 'analysis'
      timestampMs: number
      source: UiSource
      analysis: UiAnalysisPayload
    }
  | {
      kind: 'trigger'
      timestampMs: number
      source: UiSource
      trigger: UiTriggerPayload
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

export type ImuChannelRole = 'accelX' | 'accelY' | 'accelZ' | 'gyroX' | 'gyroY' | 'gyroZ'

export type ImuAttitudeRole = 'roll' | 'pitch' | 'yaw'

export type ImuQuaternionRole = 'quatW' | 'quatX' | 'quatY' | 'quatZ'

export type ImuMapMode = 'auto' | 'manual'

export type ImuOrientationSource = 'rawFusion' | 'directAngles' | 'directQuaternion'

export interface ImuChannelMap {
  accelX: string | null
  accelY: string | null
  accelZ: string | null
  gyroX: string | null
  gyroY: string | null
  gyroZ: string | null
}

export interface ImuAttitudeMap {
  roll: string | null
  pitch: string | null
  yaw: string | null
}

export interface ImuQuaternionMap {
  quatW: string | null
  quatX: string | null
  quatY: string | null
  quatZ: string | null
}

export type ImuQualityLevel = 'good' | 'warning' | 'critical' | 'idle'

export interface ImuCalibrationState {
  accelBiasApplied: boolean
  gyroBiasApplied: boolean
  sourceLabel: string | null
  lastCalibratedAtMs: number | null
}

export interface ImuQualitySnapshot {
  level: ImuQualityLevel
  label: string
  details: string
  timestampMs: number | null
}

export interface AppStatus {
  tone: 'neutral' | 'success' | 'warning' | 'danger'
  message: string
}
