export type UiConnectionState = 'idle' | 'connecting' | 'connected' | 'waitingRetry' | 'stopped'
export type UiTransportKind = 'serial' | 'fake'
export type SourceKind = 'serial' | 'fake'
export type ConsoleDisplayMode = 'ascii' | 'hex' | 'binary'

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

export type SerialDevicePortType = 'usb' | 'bluetooth' | 'pci' | 'unknown'

export interface SerialDeviceInfo {
  portName: string
  displayName: string
  portType: SerialDevicePortType
  manufacturer?: string | null
  product?: string | null
  serialNumber?: string | null
  vid?: number | null
  pid?: number | null
}

export interface McpServerStatus {
  isRunning: boolean
  transport: string
  endpointUrl?: string | null
  lastError?: string | null
}

export interface McpToolUsageSnapshot {
  name: string
  lastCalledAtMs?: number | null
}

export interface UiSource {
  transport: UiTransportKind
  port: string
  baudRate: number
}

export interface UiConnectionPayload {
  state: UiConnectionState
  reason?: string | null
  attempt: number
  retryDelayMs?: number | null
}

export interface AppStatus {
  tone: 'neutral' | 'success' | 'warning' | 'danger'
  message: string
}
