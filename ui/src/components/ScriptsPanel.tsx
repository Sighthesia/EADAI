import { useEffect, useMemo, useState } from 'react'
import { useAppStore } from '../store/appStore'
import type { HookDefinition, ScriptDefinition, UiDefinitionStatus, VariableDefinition, VariableDefinitionVisibility } from '../types'
import { buildScriptSurfaceStatus, countVariableDefinitionsBySourceKind, describeVariableSourceKind, formatTime, groupVariableDefinitionsByDevice } from './runtimeUtils'

type DefinitionLane = 'protocol' | 'hooks' | 'variables'

type DefinitionItem =
  | { lane: 'protocol'; id: string; label: string; detail: string; status: UiDefinitionStatus; updatedAtMs: number; definition: ScriptDefinition }
  | { lane: 'hooks'; id: string; label: string; detail: string; status: UiDefinitionStatus; updatedAtMs: number; definition: HookDefinition }
  | { lane: 'variables'; id: string; label: string; detail: string; status: UiDefinitionStatus; updatedAtMs: number; definition: VariableDefinition }

type DraftState = {
  name: string
  summary: string
  source: string
  language: ScriptDefinition['language']
  event: HookDefinition['event']
  enabled: boolean
  extractor: string
  bindingField: string
  sourceLabel: string
  parserName: string
  alias: string
  presentationUnit: string
  visibility: VariableDefinitionVisibility
}

export function ScriptsPanel() {
  const protocol = useAppStore((state) => state.protocol)
  const runtimeCatalog = useAppStore((state) => state.runtimeCatalog)
  const scriptDefinitions = useAppStore((state) => state.scriptDefinitions)
  const updateVariableDefinition = useAppStore((state) => state.updateVariableDefinition)

  const definitionItems = useMemo<DefinitionItem[]>(() => {
    const protocolDefinition = scriptDefinitions.scripts[0]
    const hookDefinitions = scriptDefinitions.hooks
    const variableDefinitions = scriptDefinitions.variables

    return [
      ...(protocolDefinition
        ? [
            {
              lane: 'protocol' as const,
              id: protocolDefinition.id,
              label: protocolDefinition.name,
              detail: protocolDefinition.summary,
              status: protocolDefinition.status,
              updatedAtMs: protocolDefinition.updatedAtMs,
              definition: protocolDefinition,
            },
          ]
        : []),
      ...hookDefinitions.map((definition) => ({
        lane: 'hooks' as const,
        id: definition.id,
        label: definition.name,
        detail: `${definition.event} · ${definition.enabled ? 'enabled' : 'disabled'}`,
        status: definition.status,
        updatedAtMs: definition.updatedAtMs,
        definition,
      })),
      ...variableDefinitions.map((definition) => ({
        lane: 'variables' as const,
        id: definition.id,
        label: definition.name,
        detail: `${definition.extractor} · ${definition.sourceLabel} · ${definition.visibility}`,
        status: definition.status,
        updatedAtMs: definition.updatedAtMs,
        definition,
      })),
    ]
  }, [scriptDefinitions.hooks, scriptDefinitions.scripts, scriptDefinitions.variables])

  const laneCounts = useMemo(
    () => ({
      protocol: scriptDefinitions.scripts.length,
      hooks: scriptDefinitions.hooks.length,
      variables: scriptDefinitions.variables.length,
    }),
    [scriptDefinitions.hooks.length, scriptDefinitions.scripts.length, scriptDefinitions.variables.length],
  )

  const protocolDefinition = scriptDefinitions.scripts[0] ?? null
  const definitionHooks = useMemo(() => scriptDefinitions.hooks, [scriptDefinitions.hooks])
  const variableDefinitions = useMemo(() => scriptDefinitions.variables, [scriptDefinitions.variables])
  const variableSourceCounts = useMemo(() => countVariableDefinitionsBySourceKind(variableDefinitions), [variableDefinitions])
  const definitionItemById = useMemo(() => new Map(definitionItems.map((item) => [item.id, item])), [definitionItems])
  const surfaceStatus = useMemo(
    () => buildScriptSurfaceStatus(protocolDefinition?.source ?? '', definitionHooks.length, variableDefinitions.length),
    [definitionHooks.length, protocolDefinition?.source, variableDefinitions.length],
  )

  const [selectedLane, setSelectedLane] = useState<DefinitionLane>('protocol')
  const [selectedDefinitionId, setSelectedDefinitionId] = useState<string>(protocolDefinition?.id ?? definitionItems[0]?.id ?? '')
  const [drafts, setDrafts] = useState<Record<string, DraftState>>({})

  useEffect(() => {
    if (definitionItems.length === 0) {
      setSelectedDefinitionId('')
      return
    }

    const selectedDefinitionExists = definitionItems.some((item) => item.id === selectedDefinitionId)
    if (selectedDefinitionExists) {
      return
    }

    const nextSelected = definitionItems.find((item) => item.lane === selectedLane) ?? definitionItems[0]
    setSelectedLane(nextSelected.lane)
    setSelectedDefinitionId(nextSelected.id)
  }, [definitionItems, selectedDefinitionId, selectedLane])

  const selectedDefinition = useMemo(
    () => definitionItems.find((item) => item.id === selectedDefinitionId) ?? null,
    [definitionItems, selectedDefinitionId],
  )

  useEffect(() => {
    if (!selectedDefinition) {
      return
    }

    setDrafts((current) => {
      if (current[selectedDefinition.id]) {
        return current
      }

      return {
        ...current,
        [selectedDefinition.id]: createDraft(selectedDefinition),
      }
    })
  }, [selectedDefinition])

  const selectedDraft = selectedDefinition ? drafts[selectedDefinition.id] ?? createDraft(selectedDefinition) : null
  const selectedDefinitionDirty = selectedDefinition && selectedDraft ? isDraftDirty(selectedDefinition, selectedDraft) : false

  const updateSelectedDraft = (patch: Partial<DraftState>) => {
    if (!selectedDefinition) {
      return
    }

    setDrafts((current) => ({
      ...current,
      [selectedDefinition.id]: {
        ...(current[selectedDefinition.id] ?? createDraft(selectedDefinition)),
        ...patch,
      },
    }))
  }

  const selectDefinition = (item: DefinitionItem) => {
    setSelectedLane(item.lane)
    setSelectedDefinitionId(item.id)
  }

  const resetSelectedDraft = () => {
    if (!selectedDefinition) {
      return
    }

    setDrafts((current) => ({
      ...current,
      [selectedDefinition.id]: createDraft(selectedDefinition),
    }))
  }

  const saveSelectedDraft = () => {
    if (!selectedDefinition || selectedDefinition.lane !== 'variables' || !selectedDraft) {
      return
    }

    updateVariableDefinition(selectedDefinition.id, {
      bindingField: selectedDraft.bindingField,
      alias: selectedDraft.alias.trim() || null,
      presentationUnit: selectedDraft.presentationUnit.trim() || null,
      visibility: selectedDraft.visibility,
      sourceLabel: selectedDraft.sourceLabel.trim() || selectedDefinition.definition.sourceLabel,
      summary: selectedDraft.summary.trim() || selectedDefinition.definition.summary,
    })
  }

  const selectLane = (lane: DefinitionLane) => {
    setSelectedLane(lane)
    const nextSelected = definitionItems.find((item) => item.lane === lane) ?? definitionItems[0] ?? null
    setSelectedDefinitionId(nextSelected?.id ?? '')
  }

  const selectedLaneItems = definitionItems.filter((item) => item.lane === selectedLane)
  const variableGroups = useMemo(() => groupVariableDefinitionsByDevice(variableDefinitions, runtimeCatalog.device.id), [runtimeCatalog.device.id, variableDefinitions])

  return (
    <section className="panel panel-scroll scripts-panel">
      <div className="runtime-header scripts-header">
        <div>
          <span className="mcp-label">Scripts surface</span>
          <h2>Definition browse and edit</h2>
        </div>
        <div className="runtime-header-meta">
          <span className={`metric-chip ${selectedDefinitionDirty ? 'selected' : ''}`}>{selectedDefinitionDirty ? 'Draft changed' : 'Synced'}</span>
          <span className="metric-chip">{laneCounts[selectedLane]} in lane</span>
        </div>
      </div>

      <section className="runtime-section-card scripts-overview-card">
        <div className="scripts-overview-header">
          <div>
            <span className="mcp-label">Authoring overview</span>
            <strong>{surfaceStatus.label}</strong>
            <small>{surfaceStatus.detail}</small>
          </div>
          <div className="scripts-overview-metrics">
            <span className="metric-chip">{laneCounts.protocol} protocol</span>
            <span className="metric-chip">{laneCounts.hooks} hooks</span>
            <span className="metric-chip">{laneCounts.variables} variables</span>
          </div>
        </div>
        <div className="scripts-overview-strip">
          <span className="metric-chip">{protocolDefinition?.name ?? protocol.parserName}</span>
          <span className="metric-chip">{runtimeCatalog.telemetry.fields.length} telemetry fields</span>
          <span className="metric-chip">{variableSourceCounts['protocol-text']} text · {variableSourceCounts['telemetry-sample']} samples</span>
        </div>
      </section>

      <section className="runtime-section-card scripts-browser-shell">
        <div className="protocol-schema-header">
          <strong>Browse and edit definitions</strong>
        </div>
        <div className="scripts-browser-layout">
          <aside className="scripts-browser-sidebar">
            <div className="scripts-lane-switcher" role="tablist" aria-label="Definition lanes">
              <button type="button" role="tab" aria-selected={selectedLane === 'protocol'} className={`metric-chip scripts-lane-chip ${selectedLane === 'protocol' ? 'selected' : ''}`} onClick={() => selectLane('protocol')}>
                Protocol · {laneCounts.protocol}
              </button>
              <button type="button" role="tab" aria-selected={selectedLane === 'hooks'} className={`metric-chip scripts-lane-chip ${selectedLane === 'hooks' ? 'selected' : ''}`} onClick={() => selectLane('hooks')}>
                Hooks · {laneCounts.hooks}
              </button>
              <button type="button" role="tab" aria-selected={selectedLane === 'variables'} className={`metric-chip scripts-lane-chip ${selectedLane === 'variables' ? 'selected' : ''}`} onClick={() => selectLane('variables')}>
                Variables · {laneCounts.variables}
              </button>
            </div>

            <div className="scripts-browser-group">
              <div className="protocol-schema-header">
                <strong>{laneTitle(selectedLane)}</strong>
              </div>
              <div className="scripts-browser-list">
                {selectedLaneItems.length > 0 ? (
                  selectedLane === 'variables' ? (
                    variableGroups.map((group) => (
                      <section key={group.deviceRef} className="scripts-definition-group">
                        <div className="protocol-schema-header">
                          <strong>{group.label}</strong>
                        </div>
                        <div className="scripts-browser-list scripts-browser-list-nested">
                          {group.definitions.map((item) => (
                            <button
                              key={item.id}
                              type="button"
                              className={`scripts-browser-item ${item.id === selectedDefinition?.id ? 'selected' : ''}`}
                              onClick={() => {
                                const nextSelected = definitionItemById.get(item.id)
                                if (nextSelected) {
                                  selectDefinition(nextSelected)
                                }
                              }}
                            >
                              <span className="mcp-label">Variable extraction</span>
                              <strong>{item.name}</strong>
                              <small>{item.summary}</small>
                              <small>
                                {describeVariableSourceKind(item.sourceKind)} · {item.extractorKind}
                              </small>
                               <small>{item.status} · updated {formatTime(item.updatedAtMs)}</small>
                            </button>
                          ))}
                        </div>
                      </section>
                    ))
                  ) : (
                    selectedLaneItems.map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        className={`scripts-browser-item ${item.id === selectedDefinition?.id ? 'selected' : ''}`}
                        onClick={() => selectDefinition(item)}
                      >
                        <span className="mcp-label">{item.lane === 'protocol' ? 'Protocol script' : item.lane === 'hooks' ? 'Hook definition' : 'Variable extraction'}</span>
                        <strong>{item.label}</strong>
                        <small>{item.detail}</small>
                        <small>{item.status} · updated {formatTime(item.updatedAtMs)}</small>
                      </button>
                    ))
                  )
                ) : (
                  <div className="protocol-empty">No definitions available in this lane yet.</div>
                )}
              </div>
            </div>
          </aside>

          <div className="scripts-editor-pane">
            {!selectedDefinition || !selectedDraft ? (
              <div className="protocol-empty">Select a definition to start drafting.</div>
            ) : (
              <>
                <div className="scripts-editor-header">
                    <div>
                      <span className="mcp-label">Editor</span>
                      <h3>{selectedDefinition.label}</h3>
                      <small>{selectedDefinition.detail}</small>
                    </div>
                  <div className="scripts-editor-header-actions">
                    <span className="metric-chip">{selectedDefinition.lane}</span>
                    <span className={`metric-chip ${selectedDefinitionDirty ? 'selected' : ''}`}>{selectedDefinitionDirty ? 'Draft changed' : 'Synced from store'}</span>
                  </div>
                </div>

                {selectedDefinition.lane === 'protocol' ? (
                  <div className="scripts-editor-grid">
                    <label className="scripts-editor-field">
                      <span>Name</span>
                      <input value={selectedDraft.name} onChange={(event) => updateSelectedDraft({ name: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Language</span>
                      <select value={selectedDraft.language} onChange={(event) => updateSelectedDraft({ language: event.target.value as DraftState['language'] })}>
                        <option value="typescript">TypeScript</option>
                        <option value="javascript">JavaScript</option>
                      </select>
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Summary</span>
                      <textarea rows={3} value={selectedDraft.summary} onChange={(event) => updateSelectedDraft({ summary: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Source</span>
                      <textarea className="scripts-editor-source" rows={12} value={selectedDraft.source} onChange={(event) => updateSelectedDraft({ source: event.target.value })} />
                    </label>
                  </div>
                ) : selectedDefinition.lane === 'hooks' ? (
                  <div className="scripts-editor-grid">
                    <label className="scripts-editor-field">
                      <span>Name</span>
                      <input value={selectedDraft.name} onChange={(event) => updateSelectedDraft({ name: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Event</span>
                      <select value={selectedDraft.event} onChange={(event) => updateSelectedDraft({ event: event.target.value as DraftState['event'] })}>
                        <option value="schema">schema</option>
                        <option value="sample">sample</option>
                        <option value="trigger">trigger</option>
                      </select>
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-inline">
                      <span>Enabled</span>
                      <input type="checkbox" checked={selectedDraft.enabled} onChange={(event) => updateSelectedDraft({ enabled: event.target.checked })} />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Summary</span>
                      <textarea rows={3} value={selectedDraft.summary} onChange={(event) => updateSelectedDraft({ summary: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Source</span>
                      <textarea className="scripts-editor-source" rows={12} value={selectedDraft.source} onChange={(event) => updateSelectedDraft({ source: event.target.value })} />
                    </label>
                  </div>
                ) : (
                  <div className="scripts-editor-grid">
                    <label className="scripts-editor-field">
                      <span>Name</span>
                      <input value={selectedDraft.name} onChange={(event) => updateSelectedDraft({ name: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Source label</span>
                      <input value={selectedDraft.sourceLabel} onChange={(event) => updateSelectedDraft({ sourceLabel: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Parser</span>
                      <input value={selectedDraft.parserName} onChange={(event) => updateSelectedDraft({ parserName: event.target.value })} placeholder="runtime parser name" />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Alias</span>
                      <input value={selectedDraft.alias} onChange={(event) => updateSelectedDraft({ alias: event.target.value })} placeholder="friendly variable name" />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Binding field</span>
                      <input value={selectedDraft.bindingField} onChange={(event) => updateSelectedDraft({ bindingField: event.target.value })} placeholder="parser.fields.value" />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Presentation unit</span>
                      <input value={selectedDraft.presentationUnit} onChange={(event) => updateSelectedDraft({ presentationUnit: event.target.value })} placeholder="deg, °/s, g" />
                    </label>
                    <label className="scripts-editor-field">
                      <span>Visibility</span>
                      <select value={selectedDraft.visibility} onChange={(event) => updateSelectedDraft({ visibility: event.target.value as VariableDefinitionVisibility })}>
                        <option value="both">Runtime + Variables</option>
                        <option value="runtime">Runtime only</option>
                        <option value="variables">Variables only</option>
                        <option value="hidden">Hidden</option>
                      </select>
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Summary</span>
                      <textarea rows={3} value={selectedDraft.summary} onChange={(event) => updateSelectedDraft({ summary: event.target.value })} />
                    </label>
                    <label className="scripts-editor-field scripts-editor-field-wide">
                      <span>Runtime mapping</span>
                      <textarea
                        rows={3}
                        readOnly
                        value={selectedDefinition.definition.deviceRef
                          ? `Mapped to ${selectedDefinition.definition.deviceRef} · ${selectedDefinition.definition.sourceLabel}`
                          : `Mapped to ${selectedDefinition.definition.sourceLabel}`}
                      />
                    </label>
                  </div>
                )}

                <div className="scripts-editor-toolbar">
                  <div className="scripts-editor-preview">
                    <span className="mcp-label">Runtime impact</span>
                    <small>{runtimeCatalog.telemetry.fields.length > 0 ? `${runtimeCatalog.telemetry.fields.length} telemetry fields · ${runtimeCatalog.telemetry.parserName}` : 'Telemetry schema pending'}</small>
                    <small>{selectedDefinition.lane === 'variables' ? `Observed value ${selectedDefinition.definition.lastObservedValue ?? '—'}` : `Protocol phase ${protocol.phase}`}</small>
                    {selectedDefinition.lane === 'variables' ? <small>{describeVariableSourceKind(selectedDefinition.definition.sourceKind)} · {selectedDefinition.definition.extractorKind}</small> : null}
                  </div>
                  <div className="scripts-editor-header-actions">
                    {selectedDefinition.lane === 'variables' ? (
                      <button type="button" className="ghost-button" onClick={saveSelectedDraft}>
                        Apply to runtime + variables
                      </button>
                    ) : null}
                    <button type="button" className="ghost-button" onClick={resetSelectedDraft}>
                      Reset local draft
                    </button>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      </section>
  </section>
)
}

function createDraft(item: DefinitionItem): DraftState {
  if (item.lane === 'protocol') {
    return {
      name: item.definition.name,
      summary: item.definition.summary,
      source: item.definition.source,
      language: item.definition.language,
      event: 'schema',
      enabled: true,
      extractor: '',
      bindingField: '',
      sourceLabel: 'protocol',
      parserName: '',
      alias: '',
      presentationUnit: '',
      visibility: 'hidden',
    }
  }

  if (item.lane === 'hooks') {
    return {
      name: item.definition.name,
      summary: item.definition.summary,
      source: item.definition.source,
      language: 'typescript',
      event: item.definition.event,
      enabled: item.definition.enabled,
      extractor: '',
      bindingField: '',
      sourceLabel: 'hook',
      parserName: '',
      alias: '',
      presentationUnit: '',
      visibility: 'hidden',
    }
  }

  return {
    name: item.definition.name,
    summary: item.definition.summary,
    source: item.definition.extractor,
    language: 'typescript',
    event: 'sample',
    enabled: true,
    extractor: item.definition.extractor,
    bindingField: item.definition.bindingField,
    sourceLabel: item.definition.sourceLabel,
    parserName: item.definition.parserName ?? '',
    alias: item.definition.alias ?? '',
    presentationUnit: item.definition.presentationUnit ?? '',
    visibility: item.definition.visibility,
  }
}

function isDraftDirty(item: DefinitionItem, draft: DraftState) {
  if (item.lane === 'protocol') {
    return draft.name !== item.definition.name || draft.summary !== item.definition.summary || draft.source !== item.definition.source || draft.language !== item.definition.language
  }

  if (item.lane === 'hooks') {
    return draft.name !== item.definition.name || draft.summary !== item.definition.summary || draft.source !== item.definition.source || draft.event !== item.definition.event || draft.enabled !== item.definition.enabled
  }

  return draft.name !== item.definition.name || draft.summary !== item.definition.summary || draft.extractor !== item.definition.extractor || draft.bindingField !== item.definition.bindingField || draft.sourceLabel !== item.definition.sourceLabel || draft.parserName !== (item.definition.parserName ?? '') || draft.alias !== (item.definition.alias ?? '') || draft.presentationUnit !== (item.definition.presentationUnit ?? '') || draft.visibility !== item.definition.visibility
}

function laneTitle(lane: DefinitionLane) {
  if (lane === 'protocol') return 'Protocol lane'
  if (lane === 'hooks') return 'Hook lane'
  return 'Variable lane'
}
