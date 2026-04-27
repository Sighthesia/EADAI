import { useEffect, useMemo, useState } from 'react'
import type { Bmi088HostCommand, UiProtocolHandshakePhase, UiRuntimeCatalogSnapshot, UiRuntimeCommandCatalogItem, UiRuntimeCommandParameter } from '../types'
import { RuntimeSectionHeader } from './runtimeUtils'

type ParameterDraft = Record<string, string | number | boolean>

export function RuntimeCommandCenter({
  runtimeCatalog,
  protocolPhase,
  onSendCommand,
}: {
  runtimeCatalog: UiRuntimeCatalogSnapshot
  protocolPhase: UiProtocolHandshakePhase
  onSendCommand: (command: Bmi088HostCommand, payload?: string | null) => void
}) {
  const [selectedCommand, setSelectedCommand] = useState<Bmi088HostCommand | null>(runtimeCatalog.commands[0]?.command ?? null)
  const [drafts, setDrafts] = useState<Partial<Record<Bmi088HostCommand, ParameterDraft>>>({})

  useEffect(() => {
    const commandExists = selectedCommand ? runtimeCatalog.commands.some((item) => item.command === selectedCommand) : false
    if (!commandExists && runtimeCatalog.commands[0]) {
      setSelectedCommand(runtimeCatalog.commands[0].command)
    }
  }, [runtimeCatalog.commands, selectedCommand])

  const selectedItem = useMemo(() => runtimeCatalog.commands.find((item) => item.command === selectedCommand) ?? null, [runtimeCatalog.commands, selectedCommand])

  const selectedDraft = selectedItem ? drafts[selectedItem.command] ?? buildDefaultDraft(selectedItem.parameters ?? []) : {}

  const updateDraft = (parameter: UiRuntimeCommandParameter, value: string | number | boolean) => {
    if (!selectedItem) {
      return
    }

    setDrafts((current) => ({
      ...current,
      [selectedItem.command]: {
        ...(current[selectedItem.command] ?? buildDefaultDraft(selectedItem.parameters ?? [])),
        [parameter.name]: value,
      },
    }))
  }

  return (
    <section className="runtime-section runtime-command-center">
      <RuntimeSectionHeader title="Quick actions" />
      <div className="runtime-command-strip">
        <span className="metric-chip">{runtimeCatalog.commands.length} commands</span>
        <span className="metric-chip">{protocolPhase}</span>
        <span className="metric-chip">{selectedItem?.command ?? 'no command selected'}</span>
      </div>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Command list</strong>
        </div>
        <div className="runtime-command-list" role="list" aria-label="Runtime command list">
          {runtimeCatalog.commands.map((item) => {
            const active = item.command === selectedCommand
            return (
              <article
                key={item.command}
                className={`runtime-command-row ${active ? 'active' : ''}`}
              >
                <div className="runtime-command-copy">
                  <strong>{item.command}</strong>
                  <span>{item.label}</span>
                  <small>{item.description}</small>
                </div>
                <div className="runtime-command-actions">
                  <button type="button" className="ghost-button" onClick={() => setSelectedCommand(item.command)}>
                    Details
                  </button>
                  {item.recommendedPhase === protocolPhase ? <span className="metric-chip tone-success">recommended</span> : null}
                   <button type="button" className="ghost-button" onClick={() => onSendCommand(item.command, selectedDraft.payload as string | undefined)}>
                    Send
                  </button>
                </div>
              </article>
            )
          })}
        </div>
      </section>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Selected command</strong>
          {selectedItem ? <small>{buildPayloadPreview(selectedItem, selectedDraft)}</small> : null}
        </div>
        {selectedItem ? (
          <div className="runtime-command-detail">
            <div className="runtime-command-detail-copy">
              <strong>{selectedItem.command} · {selectedItem.label}</strong>
              <small>{selectedItem.description}</small>
            </div>
            {selectedItem.parameters?.length ? (
              <div className="runtime-command-params">
                {selectedItem.parameters.map((parameter) => (
                  <RuntimeParameterEditor
                    key={parameter.name}
                    parameter={parameter}
                    value={selectedDraft[parameter.name]}
                    onChange={(value) => updateDraft(parameter, value)}
                  />
                ))}
              </div>
            ) : (
              <div className="protocol-empty">This command does not expose editable parameters yet.</div>
            )}
            <div className="toolbar-row runtime-command-detail-actions">
              <code>{buildPayloadPreview(selectedItem, selectedDraft)}</code>
              <button type="button" className="primary-button" onClick={() => onSendCommand(selectedItem.command, selectedDraft.payload as string | undefined)}>
                Send now
              </button>
            </div>
          </div>
        ) : (
          <div className="protocol-empty">No runtime commands available.</div>
        )}
      </section>
    </section>
  )
}

function RuntimeParameterEditor({
  parameter,
  value,
  onChange,
}: {
  parameter: UiRuntimeCommandParameter
  value: string | number | boolean | undefined
  onChange: (value: string | number | boolean) => void
}) {
  return (
    <label className="runtime-command-parameter">
      <span>{parameter.label}</span>
      {parameter.description ? <small>{parameter.description}</small> : null}
      {renderParameterControl(parameter, value, onChange)}
    </label>
  )
}

function renderParameterControl(
  parameter: UiRuntimeCommandParameter,
  value: string | number | boolean | undefined,
  onChange: (value: string | number | boolean) => void,
) {
  switch (parameter.kind) {
    case 'boolean':
      return (
        <label className="checkbox-row runtime-command-parameter-control">
          <input type="checkbox" checked={Boolean(value)} onChange={(event) => onChange(event.target.checked)} />
          <span>{parameter.placeholder ?? 'Enabled'}</span>
        </label>
      )
    case 'select':
      return (
        <select className="runtime-command-parameter-control" value={String(value ?? parameter.defaultValue ?? '')} onChange={(event) => onChange(event.target.value)}>
          {(parameter.options ?? []).map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      )
    case 'number':
      return (
        <input
          className="runtime-command-parameter-control"
          type="number"
          value={String(value ?? parameter.defaultValue ?? '')}
          placeholder={parameter.placeholder ?? ''}
          onChange={(event) => onChange(event.target.value === '' ? '' : Number(event.target.value))}
        />
      )
    case 'text':
    default:
      return (
        <input
          className="runtime-command-parameter-control"
          type="text"
          value={String(value ?? parameter.defaultValue ?? '')}
          placeholder={parameter.placeholder ?? ''}
          onChange={(event) => onChange(event.target.value)}
        />
      )
  }
}

function buildDefaultDraft(parameters: UiRuntimeCommandParameter[]) {
  return parameters.reduce<ParameterDraft>((accumulator, parameter) => {
    if (parameter.defaultValue !== undefined && parameter.defaultValue !== null) {
      accumulator[parameter.name] = parameter.defaultValue
    } else if (parameter.kind === 'boolean') {
      accumulator[parameter.name] = false
    } else {
      accumulator[parameter.name] = ''
    }

    return accumulator
  }, {})
}

function buildPayloadPreview(item: UiRuntimeCommandCatalogItem, draft: ParameterDraft) {
  const parameterValues = item.parameters?.map((parameter) => `${parameter.name}=${String(draft[parameter.name] ?? parameter.defaultValue ?? '')}`) ?? []
  return [item.payloadPreview ?? item.command, ...parameterValues].join(' · ')
}
