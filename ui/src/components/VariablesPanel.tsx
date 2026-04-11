import { useMemo } from 'react'
import { useAppStore } from '../store/appStore'
import type { UiAnalysisPayload, VariableEntry } from '../types'

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
                <div className="variable-subline">
                  <small>{variable.parserName ?? 'raw'}</small>
                  {variable.analysis ? <span className="metric-chip">{frequencyLabel(variable.analysis)}</span> : null}
                  {variable.analysis ? <span className="metric-chip">{dutyLabel(variable.analysis)}</span> : null}
                  <span className="metric-chip">{variable.sampleCount} samples</span>
                  {variable.latestTrigger ? (
                    <span className={`trigger-pill trigger-pill-${variable.latestTrigger.severity}`}>
                      {variable.latestTrigger.ruleId}
                    </span>
                  ) : null}
                </div>
              </div>
              <div className="variable-metric">
                <strong>{variable.currentValue}</strong>
                <small className={`trend trend-${variable.trend}`}>{trendLabel(variable)}</small>
                <small>{secondaryMetric(variable)}</small>
              </div>
            </button>
          )
        })}
      </div>
    </section>
  )
}

const frequencyLabel = (analysis: UiAnalysisPayload) =>
  analysis.frequencyHz === undefined || analysis.frequencyHz === null
    ? 'freq --'
    : `freq ${analysis.frequencyHz.toFixed(2)}Hz`

const dutyLabel = (analysis: UiAnalysisPayload) =>
  analysis.dutyCycle === undefined || analysis.dutyCycle === null
    ? 'duty --'
    : `duty ${analysis.dutyCycle.toFixed(0)}%`

const secondaryMetric = (variable: VariableEntry) => {
  if (variable.latestTrigger) {
    return variable.latestTrigger.reason
  }
  if (variable.analysis?.rmsValue !== undefined && variable.analysis?.rmsValue !== null) {
    return `rms ${variable.analysis.rmsValue.toFixed(3)}`
  }
  if (variable.analysis?.meanValue !== undefined && variable.analysis?.meanValue !== null) {
    return `mean ${variable.analysis.meanValue.toFixed(3)}`
  }
  return `${variable.triggerCount} triggers`
}

const trendLabel = (variable: VariableEntry) => {
  if (variable.analysis?.changeRate !== undefined && variable.analysis?.changeRate !== null) {
    return `${variable.trend} · ${variable.analysis.changeRate.toFixed(2)}/s`
  }
  return variable.trend
}
