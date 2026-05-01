import { useEffect, useMemo, useState } from 'react'
import { useAppStore } from '../store/appStore'
import type { HookDefinition, UiDefinitionStatus, VariableDefinition } from '../types'
import { describeVariableSourceKind, formatTime } from './runtimeUtils'

type DefinitionLane = 'hooks' | 'signals'

type DefinitionItem =
  | { lane: 'hooks'; id: string; label: string; detail: string; status: UiDefinitionStatus; updatedAtMs: number; definition: HookDefinition }
  | { lane: 'signals'; id: string; label: string; detail: string; status: UiDefinitionStatus; updatedAtMs: number; definition: VariableDefinition }

export function ScriptsPanel() {
  const runtimeCatalog = useAppStore((state) => state.runtimeCatalog)
  const scriptDefinitions = useAppStore((state) => state.scriptDefinitions)

  const definitionItems = useMemo<DefinitionItem[]>(() => {
    return [
      ...scriptDefinitions.hooks.map((definition) => ({
        lane: 'hooks' as const,
        id: definition.id,
        label: definition.name,
        detail: `${definition.event} · ${definition.enabled ? 'enabled' : 'disabled'}`,
        status: definition.status,
        updatedAtMs: definition.updatedAtMs,
        definition,
      })),
      ...scriptDefinitions.variables.map((definition) => ({
        lane: 'signals' as const,
        id: definition.id,
        label: definition.name,
        detail: `${definition.extractorKind} · ${definition.sourceLabel} · ${definition.visibility}`,
        status: definition.status,
        updatedAtMs: definition.updatedAtMs,
        definition,
      })),
    ]
  }, [scriptDefinitions.hooks, scriptDefinitions.variables])

  const laneCounts = useMemo(
    () => ({
      hooks: scriptDefinitions.hooks.length,
      signals: scriptDefinitions.variables.length,
    }),
    [scriptDefinitions.hooks.length, scriptDefinitions.variables.length],
  )

  const signalDefinitions = useMemo(() => scriptDefinitions.variables, [scriptDefinitions.variables])
  const hookDefinitions = useMemo(() => scriptDefinitions.hooks, [scriptDefinitions.hooks])
  const definitionItemById = useMemo(() => new Map(definitionItems.map((item) => [item.id, item])), [definitionItems])
  const signalSourceCounts = useMemo(
    () => ({
      'protocol-text': signalDefinitions.filter((definition) => definition.sourceKind === 'protocol-text').length,
      'telemetry-sample': signalDefinitions.filter((definition) => definition.sourceKind === 'telemetry-sample').length,
    }),
    [signalDefinitions],
  )

  const [selectedLane, setSelectedLane] = useState<DefinitionLane>('signals')
  const [selectedDefinitionId, setSelectedDefinitionId] = useState<string>(definitionItems[0]?.id ?? '')

  useEffect(() => {
    if (definitionItems.length === 0) {
      setSelectedDefinitionId('')
      return
    }

    if (definitionItems.some((item) => item.id === selectedDefinitionId)) {
      return
    }

    const nextSelected = definitionItems.find((item) => item.lane === selectedLane) ?? definitionItems[0]
    setSelectedLane(nextSelected.lane)
    setSelectedDefinitionId(nextSelected.id)
  }, [definitionItems, selectedDefinitionId, selectedLane])

  const selectedDefinition = useMemo(() => definitionItems.find((item) => item.id === selectedDefinitionId) ?? null, [definitionItems, selectedDefinitionId])
  const selectedLaneItems = definitionItems.filter((item) => item.lane === selectedLane)

  return (
    <section className="panel panel-scroll scripts-panel">
      <div className="runtime-header scripts-header">
        <div>
          <span className="mcp-label">Automation surface</span>
          <h2>Signals and hook seeds</h2>
        </div>
        <div className="runtime-header-meta">
          <span className="metric-chip">{laneCounts.signals} signals</span>
          <span className="metric-chip">{laneCounts.hooks} hooks</span>
        </div>
      </div>

      <section className="runtime-section-card scripts-overview-card">
        <div className="scripts-overview-header">
          <div>
            <span className="mcp-label">Observed data</span>
            <strong>Runtime-derived signals stay separate from automation</strong>
            <small>Built-in protocol state lives in the runtime panels; this surface only shows reusable signal extracts and hook seeds.</small>
          </div>
          <div className="scripts-overview-metrics">
            <span className="metric-chip">{runtimeCatalog.telemetry.fields.length} telemetry fields</span>
            <span className="metric-chip">{signalSourceCounts['protocol-text']} text</span>
            <span className="metric-chip">{signalSourceCounts['telemetry-sample']} samples</span>
          </div>
        </div>
      </section>

      <section className="runtime-section-card scripts-browser-shell">
        <div className="protocol-schema-header">
          <strong>Browse signals and automation</strong>
        </div>
        <div className="scripts-browser-layout">
          <aside className="scripts-browser-sidebar">
            <div className="scripts-lane-switcher" role="tablist" aria-label="Automation lanes">
              <button
                type="button"
                role="tab"
                aria-selected={selectedLane === 'signals'}
                className={`metric-chip scripts-lane-chip ${selectedLane === 'signals' ? 'selected' : ''}`}
                onClick={() => {
                  setSelectedLane('signals')
                  const nextSelected = definitionItems.find((item) => item.lane === 'signals') ?? definitionItems[0]
                  setSelectedDefinitionId(nextSelected?.id ?? '')
                }}
              >
                Signals · {laneCounts.signals}
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={selectedLane === 'hooks'}
                className={`metric-chip scripts-lane-chip ${selectedLane === 'hooks' ? 'selected' : ''}`}
                onClick={() => {
                  setSelectedLane('hooks')
                  const nextSelected = definitionItems.find((item) => item.lane === 'hooks') ?? definitionItems[0]
                  setSelectedDefinitionId(nextSelected?.id ?? '')
                }}
              >
                Hooks · {laneCounts.hooks}
              </button>
            </div>

            <div className="scripts-browser-group">
              <div className="protocol-schema-header">
                <strong>{selectedLane === 'signals' ? 'Signals lane' : 'Hooks lane'}</strong>
              </div>
              <div className="scripts-browser-list">
                {selectedLaneItems.length > 0 ? (
                  selectedLane === 'signals' ? (
                    signalDefinitions.map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        className={`scripts-browser-item ${item.id === selectedDefinition?.id ? 'selected' : ''}`}
                        onClick={() => {
                          const nextSelected = definitionItemById.get(item.id)
                          if (nextSelected) {
                            setSelectedLane(nextSelected.lane)
                            setSelectedDefinitionId(nextSelected.id)
                          }
                        }}
                      >
                        <span className="mcp-label">Signal</span>
                        <strong>{item.name}</strong>
                        <small>{item.summary}</small>
                        <small>
                          {describeVariableSourceKind(item.sourceKind)} · {item.extractorKind}
                        </small>
                        <small>{item.status} · updated {formatTime(item.updatedAtMs)}</small>
                      </button>
                    ))
                  ) : (
                    hookDefinitions.map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        className={`scripts-browser-item ${item.id === selectedDefinition?.id ? 'selected' : ''}`}
                        onClick={() => {
                          const nextSelected = definitionItemById.get(item.id)
                          if (nextSelected) {
                            setSelectedLane(nextSelected.lane)
                            setSelectedDefinitionId(nextSelected.id)
                          }
                        }}
                      >
                        <span className="mcp-label">Hook</span>
                        <strong>{item.name}</strong>
                        <small>{item.event} · {item.enabled ? 'enabled' : 'disabled'}</small>
                        <small>{item.status} · updated {formatTime(item.updatedAtMs)}</small>
                      </button>
                    ))
                  )
                ) : (
                  <div className="protocol-empty">No automation entries available in this lane yet.</div>
                )}
              </div>
            </div>
          </aside>

          <div className="scripts-editor-pane">
            {!selectedDefinition ? (
              <div className="protocol-empty">Select a signal or hook to inspect its details.</div>
            ) : (
              <>
                <div className="scripts-editor-header">
                  <div>
                    <span className="mcp-label">Inspector</span>
                    <h3>{selectedDefinition.label}</h3>
                    <small>{selectedDefinition.detail}</small>
                  </div>
                  <div className="scripts-editor-header-actions">
                    <span className="metric-chip">{selectedDefinition.lane}</span>
                    <span className="metric-chip">{selectedDefinition.status}</span>
                  </div>
                </div>

                {selectedDefinition.lane === 'signals' ? (
                  <div className="scripts-editor-grid">
                    <label className="scripts-editor-field">
                      <span>Source label</span>
                      <input value={selectedDefinition.definition.sourceLabel} readOnly />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Extractor</span>
                      <input value={selectedDefinition.definition.extractor} readOnly />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Binding field</span>
                      <input value={selectedDefinition.definition.bindingField} readOnly />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Visibility</span>
                      <input value={selectedDefinition.definition.visibility} readOnly />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Summary</span>
                      <textarea rows={3} value={selectedDefinition.definition.summary} readOnly />
                    </label>
                  </div>
                ) : (
                  <div className="scripts-editor-grid">
                    <label className="scripts-editor-field">
                      <span>Event</span>
                      <input value={selectedDefinition.definition.event} readOnly />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Enabled</span>
                      <input value={selectedDefinition.definition.enabled ? 'yes' : 'no'} readOnly />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Summary</span>
                      <textarea rows={3} value={selectedDefinition.definition.summary} readOnly />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Example snippet</span>
                      <textarea className="scripts-editor-source" rows={10} value={selectedDefinition.definition.source} readOnly />
                    </label>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </section>
    </section>
  )
}
