export interface LogicAnalyzerCaptureRequest {
  deviceRef: string
  sampleCount: number
  samplerateHz?: number | null
  channels: string[]
}

export interface LogicAnalyzerConfig {
  deviceRef: string
  sampleCount: number
  samplerateHzInput: string
  selectedChannelLabels: string[]
}

export interface LogicAnalyzerDevice {
  reference: string
  name: string
  driver?: string | null
  channels: string[]
  note?: string | null
  rawLine?: string | null
}

export type LogicAnalyzerSessionState =
  | 'unavailable'
  | 'idle'
  | 'scanning'
  | 'ready'
  | 'capturing'
  | 'stopping'
  | 'error'

export interface LogicAnalyzerCaptureState {
  pid: number
  startedAtMs: number
  command: string
  outputPath: string
}

export interface LogicAnalyzerWaveformChannel {
  label: string
  samples: Array<boolean | null>
}

export interface LogicAnalyzerCaptureResult {
  outputPath: string
  sampleRateHz?: number | null
  sampleCount: number
  channels: LogicAnalyzerWaveformChannel[]
  capturedAtMs: number
}

export interface LogicAnalyzerStatus {
  available: boolean
  executable?: string | null
  sessionState: LogicAnalyzerSessionState
  devices: LogicAnalyzerDevice[]
  selectedDeviceRef?: string | null
  activeCapture?: LogicAnalyzerCaptureState | null
  lastCapture?: LogicAnalyzerCaptureResult | null
  lastScanAtMs?: number | null
  scanOutput?: string | null
  lastError?: string | null
  capturePlan?: string | null
  linuxFirstNote: string
}
