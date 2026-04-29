import type { LogicAnalyzerConfig, LogicAnalyzerStatus } from '../types'
import { getLogicAnalyzerStatus, refreshLogicAnalyzerDevices } from '../lib/tauri'
import { DEFAULT_LOGIC_SAMPLE_COUNT } from './constants'

export const defaultLogicAnalyzerStatus = (): LogicAnalyzerStatus => ({
  available: false,
  executable: null,
  sessionState: 'idle',
  devices: [],
  selectedDeviceRef: null,
  activeCapture: null,
  lastCapture: null,
  lastScanAtMs: null,
  scanOutput: null,
  lastError: null,
  capturePlan: null,
  linuxFirstNote: 'Linux-first sigrok path; install sigrok-cli or set EADAI_SIGROK_CLI.',
})

export const syncLogicAnalyzerConfig = (
  config: LogicAnalyzerConfig,
  logicAnalyzer: LogicAnalyzerStatus,
): LogicAnalyzerConfig => {
  const availableDeviceRefs = new Set(logicAnalyzer.devices.map((device) => device.reference))
  const nextDeviceRef =
    config.deviceRef && availableDeviceRefs.has(config.deviceRef)
      ? config.deviceRef
      : logicAnalyzer.selectedDeviceRef ?? logicAnalyzer.devices[0]?.reference ?? ''

  const availableChannelLabels = new Set(logicAnalyzer.lastCapture?.channels.map((channel) => channel.label) ?? [])
  const nextChannels = config.selectedChannelLabels.filter((channel) => availableChannelLabels.has(channel))

  return {
    ...config,
    deviceRef: nextDeviceRef,
    selectedChannelLabels:
      nextChannels.length > 0
        ? nextChannels
        : logicAnalyzer.lastCapture?.channels.map((channel) => channel.label) ?? config.selectedChannelLabels,
  }
}

export const parseLogicSamplerate = (value: string) => {
  const parsed = Number(value.trim())
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

export const readLogicAnalyzerStatusSafely = async () => {
  try {
    return await getLogicAnalyzerStatus()
  } catch {
    return defaultLogicAnalyzerStatus()
  }
}

export const refreshLogicAnalyzerDevicesSafely = async () => {
  try {
    return await refreshLogicAnalyzerDevices()
  } catch {
    return readLogicAnalyzerStatusSafely()
  }
}
