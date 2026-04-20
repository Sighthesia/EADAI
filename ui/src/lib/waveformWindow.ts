export const MIN_WAVEFORM_WINDOW_MS = 2_000
export const MAX_WAVEFORM_WINDOW_MS = 120_000
export const DEFAULT_WAVEFORM_WINDOW_MS = 15_000

export function clampWaveformWindowMs(value: number) {
  return Math.min(MAX_WAVEFORM_WINDOW_MS, Math.max(MIN_WAVEFORM_WINDOW_MS, value))
}

export function scaleWaveformWindowMs(current: number, factor: number) {
  return clampWaveformWindowMs(Math.round(current * factor))
}

export function formatWaveformWindowMs(value: number) {
  if (value >= 60_000) {
    return `${(value / 60_000).toFixed(value % 60_000 === 0 ? 0 : 1)} min`
  }

  return `${(value / 1_000).toFixed(value % 1_000 === 0 ? 0 : 1)} s`
}
