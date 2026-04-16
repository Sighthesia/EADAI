export const WAVEFORM_VISUAL_AID_KEYS = ['labels', 'range', 'mean', 'median', 'period', 'slope', 'text'] as const

export type WaveformVisualAidKey = (typeof WAVEFORM_VISUAL_AID_KEYS)[number]

export type WaveformVisualAidPreferences = Partial<Record<WaveformVisualAidKey, boolean>>

export type WaveformVisualAidState = Record<string, WaveformVisualAidPreferences>

const VISUAL_AID_STORAGE_KEY = 'waveform.visual-aid-state.v1'

export function readWaveformVisualAidState(): WaveformVisualAidState {
  if (typeof window === 'undefined') {
    return {}
  }

  try {
    const raw = window.localStorage.getItem(VISUAL_AID_STORAGE_KEY)
    if (!raw) {
      return {}
    }

    const parsed = JSON.parse(raw) as Record<string, unknown>
    return Object.fromEntries(
      Object.entries(parsed)
        .map(([channel, value]) => [channel, sanitizeWaveformVisualAidPreferences(value)])
        .filter(([, value]) => Object.keys(value).length > 0),
    ) as WaveformVisualAidState
  } catch {
    return {}
  }
}

export function writeWaveformVisualAidState(state: WaveformVisualAidState) {
  if (typeof window === 'undefined') {
    return
  }

  try {
    window.localStorage.setItem(VISUAL_AID_STORAGE_KEY, JSON.stringify(state))
  } catch {
    // Ignore storage failures so the UI still works in restricted browser modes.
  }
}

export function isWaveformVisualAidEnabled(state: WaveformVisualAidState, channel: string, key: WaveformVisualAidKey) {
  return state[channel]?.[key] ?? true
}

export function setWaveformVisualAidPreference(
  state: WaveformVisualAidState,
  channel: string,
  key: WaveformVisualAidKey,
  enabled: boolean,
): WaveformVisualAidState {
  const current = state[channel] ?? {}

  if (enabled) {
    const { [key]: _removed, ...restPreferences } = current
    if (Object.keys(restPreferences).length === 0) {
      const { [channel]: _removedChannel, ...restState } = state
      return restState
    }

    return {
      ...state,
      [channel]: restPreferences,
    }
  }

  return {
    ...state,
    [channel]: {
      ...current,
      [key]: false,
    },
  }
}

function sanitizeWaveformVisualAidPreferences(value: unknown): WaveformVisualAidPreferences {
  if (typeof value === 'boolean') {
    return value ? {} : Object.fromEntries(WAVEFORM_VISUAL_AID_KEYS.map((key) => [key, false])) as WaveformVisualAidPreferences
  }

  if (!value || typeof value !== 'object') {
    return {}
  }

  const objectValue = value as Record<string, unknown>
  return Object.fromEntries(
    WAVEFORM_VISUAL_AID_KEYS.flatMap((key) => (typeof objectValue[key] === 'boolean' ? [[key, objectValue[key]]] : [])),
  ) as WaveformVisualAidPreferences
}
