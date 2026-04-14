import { useMemo, useState } from 'react'
import { useAppStore } from '../store/appStore'
import type { UiAnalysisPayload, VariableEntry } from '../types'

export function VariablesPanel() {
  const variables = useAppStore((state) => state.variables)
  const selectedChannels = useAppStore((state) => state.selectedChannels)
  const toggleChannel = useAppStore((state) => state.toggleChannel)
  const colorForChannel = useAppStore((state) => state.colorForChannel)
  const [copiedChannel, setCopiedChannel] = useState<string | null>(null)
  const rows = useMemo(
    () => Object.values(variables).sort((left, right) => right.updatedAtMs - left.updatedAtMs),
    [variables],
  )

  const copyAnalysisJson = async (variable: VariableEntry) => {
    if (!variable.analysis) {
      return
    }

    try {
      await navigator.clipboard.writeText(JSON.stringify(variable.analysis, null, 2))
      setCopiedChannel(variable.name)
      window.setTimeout(() => {
        setCopiedChannel((current) => (current === variable.name ? null : current))
      }, 1400)
    } catch {
      setCopiedChannel(null)
    }
  }

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
            <article
              key={variable.name}
              className={`variable-card ${selected ? 'selected' : ''}`}
              role="button"
              tabIndex={0}
              onClick={() => toggleChannel(variable.name)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  toggleChannel(variable.name)
                }
              }}
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
                {variable.analysis ? (
                  <div className="variable-analysis-grid">
                    <small>{rangeLabel(variable.analysis)}</small>
                    <small>{varianceLabel(variable.analysis)}</small>
                    <small>{periodLabel(variable.analysis)}</small>
                    <small>{spanLabel(variable.analysis)}</small>
                  </div>
                ) : null}
              </div>
              <div className="variable-metric">
                <strong>{variable.currentValue}</strong>
                <small className={`trend trend-${variable.trend}`}>{trendLabel(variable)}</small>
                <small>{secondaryMetric(variable)}</small>
                {variable.analysis ? (
                  <button
                    className="ghost-button variable-card-action"
                    onClick={(event) => {
                      event.stopPropagation()
                      void copyAnalysisJson(variable)
                    }}
                  >
                    {copiedChannel === variable.name ? 'Copied' : 'Copy Analysis JSON'}
                  </button>
                ) : null}
              </div>
            </article>
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

const rangeLabel = (analysis: UiAnalysisPayload) => {
  if (analysis.minValue === undefined || analysis.minValue === null || analysis.maxValue === undefined || analysis.maxValue === null) {
    return 'range --'
  }
  return `range ${analysis.minValue.toFixed(2)}..${analysis.maxValue.toFixed(2)}`
}

const varianceLabel = (analysis: UiAnalysisPayload) =>
  analysis.variance === undefined || analysis.variance === null
    ? 'variance --'
    : `variance ${analysis.variance.toFixed(3)}`

const periodLabel = (analysis: UiAnalysisPayload) =>
  analysis.periodStability === undefined || analysis.periodStability === null
    ? 'stability --'
    : `stability ${(analysis.periodStability * 100).toFixed(0)}%`

const spanLabel = (analysis: UiAnalysisPayload) =>
  analysis.timeSpanMs === undefined || analysis.timeSpanMs === null
    ? 'span --'
    : `span ${analysis.timeSpanMs.toFixed(0)}ms`

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
