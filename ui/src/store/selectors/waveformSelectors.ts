import type { VariableEntry } from '../../types'
import { useAppStore } from '../appStore'

type AppStoreState = ReturnType<typeof useAppStore.getState>

export type SelectedWaveformVariable = {
  name: string
  color: string
  variable: VariableEntry
}

/**
 * Return selected variables in channel order with stable color mapping.
 * This selector is designed for `useShallow` so WaveformPanel only rerenders
 * when selected channel entries or their references actually change.
 */
export const selectSelectedWaveformVariables = (
  state: Pick<AppStoreState, 'selectedChannels' | 'variables' | 'colorForChannel'>,
): SelectedWaveformVariable[] =>
  state.selectedChannels.reduce<SelectedWaveformVariable[]>((acc, channel) => {
    const variable = state.variables[channel]
    if (!variable) {
      return acc
    }

    acc.push({
      name: variable.name,
      color: state.colorForChannel(variable.name),
      variable,
    })
    return acc
  }, [])
