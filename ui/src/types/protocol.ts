import type { SourceKind, SerialDeviceInfo } from './session'

export type Bmi088HostCommand =
  | 'ACK'
  | 'START'
  | 'STOP'
  | 'REQ_SCHEMA'
  | 'REQ_IDENTITY'
  | 'REQ_TUNING'
  | 'SET_TUNING'
  | 'SHELL_EXEC'

export type UiRuntimeCommandParameterKind = 'text' | 'number' | 'boolean' | 'select'

export interface UiRuntimeCommandParameterOption {
  label: string
  value: string
}

export interface UiRuntimeCommandParameter {
  name: string
  label: string
  kind: UiRuntimeCommandParameterKind
  description?: string | null
  placeholder?: string | null
  defaultValue?: string | number | boolean | null
  required?: boolean
  options?: UiRuntimeCommandParameterOption[]
}

export interface Bmi088CommandRequest {
  command: Bmi088HostCommand
  payload?: string | null
}

export interface UiTelemetrySchemaField {
  name: string
  unit: string
  scaleQ: number
}

export interface UiTelemetrySchemaPayload {
  rateHz: number
  sampleLen: number
  fields: UiTelemetrySchemaField[]
}

export interface UiTelemetryIdentityPayload {
  identityFormatVersion: number
  deviceName: string
  boardName: string
  firmwareVersion: string
  protocolName: string
  protocolVersion: string
  transportName: string
  sampleRateHz: number
  schemaFieldCount: number
  samplePayloadLen: number
  protocolVersionByte: number
  featureFlags: number
  baudRate: number
  protocolMinorVersion: number
}

export interface UiTelemetrySampleField {
  name: string
  raw: number
  value: number
  unit?: string | null
  scaleQ: number
  index: number
}

export interface UiTelemetrySamplePayload {
  fields: UiTelemetrySampleField[]
}

export interface UiShellOutputPayload {
  text: string
  raw: number[]
  rawLength: number
  direction: 'rx' | 'tx'
}

export type UiProtocolHandshakePhase = 'awaitingIdentity' | 'awaitingSchema' | 'awaitingAck' | 'awaitingStart' | 'streaming' | 'stopped'

export interface UiProtocolHandshakeEvent {
  timestampMs: number
  direction: 'tx' | 'rx'
  command: 'ACK' | 'START' | 'STOP' | 'REQ_SCHEMA' | 'REQ_IDENTITY' | 'REQ_TUNING' | 'SET_TUNING' | 'SHELL_EXEC' | 'SHELL_OUTPUT' | 'SCHEMA' | 'IDENTITY' | 'SAMPLE'
  note: string
  parserStatus: 'unparsed' | 'parsed' | 'malformed'
}

export interface UiProtocolSnapshot {
  active: boolean
  parserName: string
  transportLabel: string
  baudRate: number
  phase: UiProtocolHandshakePhase
  identity?: UiTelemetryIdentityPayload | null
  schema?: UiTelemetrySchemaPayload | null
  lastPacketKind?: 'schema' | 'sample' | 'command' | null
  lastPacketRawFrame?: number[] | null
  lastSchemaAtMs?: number | null
  lastSampleAtMs?: number | null
  lastHandshakeAtMs?: number | null
  timeline: UiProtocolHandshakeEvent[]
}

export interface UiRuntimeCommandCatalogItem {
  command: Bmi088HostCommand
  label: string
  description: string
  recommendedPhase: UiProtocolHandshakePhase | null
  parameters?: UiRuntimeCommandParameter[]
  payloadPreview?: string | null
}

export interface UiRuntimeTelemetryCatalogField {
  name: string
  unit: string
  scaleQ: number
  index: number
}

export interface UiRuntimeTelemetryCatalogSnapshot {
  parserName: string
  rateHz: number | null
  sampleLen: number | null
  fields: UiRuntimeTelemetryCatalogField[]
  lastSchemaAtMs: number | null
  lastSampleAtMs: number | null
}

export interface UiRuntimeDeviceSnapshot {
  id: string
  label: string
  detail: string
  status: string
  transportLabel: string
  portLabel: string | null
  baudRate: number | null
  sourceKind: SourceKind
  connected: boolean
  serialDevice?: SerialDeviceInfo | null
}

export interface UiRuntimeCatalogSnapshot {
  device: UiRuntimeDeviceSnapshot
  commands: UiRuntimeCommandCatalogItem[]
  telemetry: UiRuntimeTelemetryCatalogSnapshot
}
