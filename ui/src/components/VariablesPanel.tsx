import { useMemo } from 'react'
import { useAppStore } from '../store/appStore'

export function VariablesPanel() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const toggleChannel = useAppStore((state) => state.toggleChannel)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const rows = useMemo(
    () => Object.values(variables).sort((left, right) => right.updatedAtMs - left.updatedAtMs),
    [variables],
  )

  return (
    <section className="panel panel-scroll">
      <div className="variables-header">
        <span>Auto-discovered channels</span>
        <span>{rows.length}</span>
      </div>
      <div className="variables-list">
        {rows.map((variable) => {
          const selected = selectedChannels.includes(variable.name)
          return (
            <button
              key={variable.name}
              className={`variable-card ${selected ? 'selected' : ''}`}
              onClick={() => toggleChannel(variable.name)}
            >
              <span className="variable-color" style={{ backgroundColor: colorForChannel(variable.name) }} />
              <div className="variable-main">
                <strong>{variable.name}</strong>
                <small>{variable.parserName ?? 'raw'}</small>
              </div>
              <div className="variable-metric">
                <strong>{variable.currentValue}</strong>
                <small className={`trend trend-${variable.trend}`}>{variable.trend}</small>
              </div>
            </button>
          )
        })}
      </div>
    </section>
  )
}
