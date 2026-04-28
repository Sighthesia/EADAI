import { useEffect, useMemo, useRef, useState } from 'react'
import type { Bmi088HostCommand, ConsoleDisplayMode, ConsoleEntry, UiProtocolHandshakePhase, UiRuntimeCatalogSnapshot } from '../types'
import { buildRuntimeCommandTemplateSelection, formatBytes, formatEntry, formatTime } from './runtimeUtils'

type PacketGroup = {
  key: string
  entries: ConsoleEntry[]
  shapeLabel: string
  shapeTokens: PacketToken[]
}

type PacketToken = {
  kind: 'text' | 'number'
  value: string
  numericValue?: number
}

export function RuntimeConsoleSection({
  runtimeCatalog,
  protocolPhase,
  commandInput,
  consoleDisplayMode,
  consoleEntries,
  sentConsoleEntries,
  appendNewline,
  onCommandInputChange,
  onAppendNewlineChange,
  onConsoleDisplayModeChange,
  onSend,
  onSendCommand,
}: {
  runtimeCatalog: UiRuntimeCatalogSnapshot
  protocolPhase: UiProtocolHandshakePhase
  commandInput: string
  consoleDisplayMode: ConsoleDisplayMode
  consoleEntries: ConsoleEntry[]
  sentConsoleEntries: ConsoleEntry[]
  appendNewline: boolean
  onCommandInputChange: (value: string) => void
  onAppendNewlineChange: (value: boolean) => void
  onConsoleDisplayModeChange: (value: ConsoleDisplayMode) => void
  onSend: () => void
  onSendCommand: (command: Bmi088HostCommand, payload?: string | null) => void
}) {
  const [selectedCommand, setSelectedCommand] = useState<Bmi088HostCommand | null>(runtimeCatalog.commands[0]?.command ?? null)
  const commandInputRef = useRef<HTMLTextAreaElement | null>(null)

  useEffect(() => {
    const commandExists = selectedCommand ? runtimeCatalog.commands.some((item) => item.command === selectedCommand) : false
    if (!commandExists) {
      setSelectedCommand(runtimeCatalog.commands[0]?.command ?? null)
    }
  }, [runtimeCatalog.commands, selectedCommand])

  const selectedCommandItem = useMemo(
    () => runtimeCatalog.commands.find((item) => item.command === selectedCommand) ?? null,
    [runtimeCatalog.commands, selectedCommand],
  )

  const receivedEntries = useMemo(() => consoleEntries.filter((entry) => entry.direction === 'rx'), [consoleEntries])
  const receivedGroups = useMemo(() => buildPacketGroups(receivedEntries, consoleDisplayMode), [consoleDisplayMode, receivedEntries])
  const sentEntries = useMemo(() => sentConsoleEntries.slice().reverse(), [sentConsoleEntries])
  const commandTemplate = useMemo(() => (selectedCommandItem ? buildRuntimeCommandTemplateSelection(selectedCommandItem) : null), [selectedCommandItem])

  const applyCommandTemplate = (item: (typeof runtimeCatalog.commands)[number]) => {
    setSelectedCommand(item.command)

    if (item.parameters?.length === 0) {
      onCommandInputChange('')
      onSendCommand(item.command)
      return
    }

    const { template, selection } = buildRuntimeCommandTemplateSelection(item)
    onCommandInputChange(template)

    window.requestAnimationFrame(() => {
      const input = commandInputRef.current
      if (!input) return

      input.focus()
      input.setSelectionRange(selection[0], selection[1])
    })
  }

  const sendSelectedCommand = () => {
    if (!selectedCommandItem) {
      onSend()
      return
    }

    if ((selectedCommandItem.parameters?.length ?? 0) === 0) {
      onSendCommand(selectedCommandItem.command)
      return
    }

    const payload = commandInput.trim()
    onSendCommand(selectedCommandItem.command, payload.length > 0 ? payload : null)
  }

  return (
    <section className="runtime-section runtime-terminal-shell">
      <div className="runtime-terminal-workspace">
        <section className="runtime-section-card runtime-terminal-half runtime-terminal-receive-half">
          <div className="protocol-schema-header">
            <div>
              <strong>Receive area</strong>
              <small>{receivedEntries.length > 0 ? `${receivedEntries.length} serial lines` : 'Waiting for RX traffic'}</small>
            </div>
            <div className="runtime-terminal-header-actions">
              <span className="metric-chip">{protocolPhase}</span>
              <div className="runtime-terminal-mode-switch" role="group" aria-label="Terminal payload display mode">
                {(['ascii', 'hex', 'binary'] as const).map((mode) => (
                  <button
                    key={mode}
                    type="button"
                    className={`metric-display-button ${consoleDisplayMode === mode ? 'active' : ''}`}
                    onClick={() => onConsoleDisplayModeChange(mode)}
                  >
                    {mode === 'ascii' ? 'ASCII' : mode.toUpperCase()}
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="runtime-terminal-stack runtime-terminal-receive-stack">
            <div className="runtime-entry-list runtime-console-entry-list runtime-terminal-entry-list runtime-terminal-fill-list">
              {receivedEntries.length > 0 ? (
                consoleDisplayMode === 'ascii' ? (
                  receivedGroups.map((group) => <RuntimePacketGroupRow key={group.key} group={group} />)
                ) : (
                  receivedEntries.map((entry, index) => (
                    <RuntimeTrafficRow key={`${entry.id}-${index}`} entry={entry} mode={consoleDisplayMode} />
                  ))
                )
              ) : (
                <div className="protocol-empty">Awaiting serial receive traffic.</div>
              )}
            </div>
          </div>
        </section>

        <section className="runtime-section-card runtime-terminal-half runtime-terminal-send-half">
          <div className="protocol-schema-header">
            <div>
              <strong>Send area</strong>
              <small>{selectedCommandItem ? `${selectedCommandItem.command} selected` : 'Choose a command or send raw text'}</small>
            </div>
            <span className={`metric-chip ${appendNewline ? 'selected' : ''}`}>{appendNewline ? 'newline on' : 'newline off'}</span>
          </div>

          <div className="runtime-terminal-stack runtime-terminal-send-stack">
            <div className="runtime-entry-list runtime-console-entry-list runtime-terminal-entry-list runtime-terminal-fill-list">
              {sentEntries.length > 0 ? sentEntries.map((entry, index) => <RuntimeTrafficRow key={`${entry.id}-${index}`} entry={entry} mode={consoleDisplayMode} />) : <div className="protocol-empty">Awaiting sent traffic.</div>}
            </div>

            <details className="runtime-disclosure runtime-terminal-command-disclosure" open>
              <summary>
                <strong>Command list</strong>
                <small>{runtimeCatalog.commands.length} commands</small>
              </summary>
              <div className="runtime-disclosure-body">
                <div className="runtime-terminal-command-list" role="list" aria-label="Runtime command list">
                  {runtimeCatalog.commands.length > 0 ? (
                    runtimeCatalog.commands.map((item) => {
                      const active = item.command === selectedCommand
                      return (
                        <button key={item.command} type="button" className={`runtime-terminal-command-row ${active ? 'active' : ''}`} onClick={() => applyCommandTemplate(item)}>
                          <div className="runtime-command-copy">
                            <strong>{item.command}</strong>
                            <span>{item.label}</span>
                            <small>{item.description}</small>
                          </div>
                          <div className="runtime-terminal-command-meta">
                            {item.recommendedPhase === protocolPhase ? <span className="metric-chip tone-success">recommended</span> : null}
                            {item.parameters?.length ? <span className="metric-chip">template</span> : <span className="metric-chip">send now</span>}
                          </div>
                        </button>
                      )
                    })
                  ) : (
                    <div className="protocol-empty">No command catalog entries available yet.</div>
                  )}
                </div>
              </div>
            </details>

            <section className="runtime-section-card runtime-terminal-compose-card">
              <div className="console-compose runtime-console-compose">
                <textarea
                  ref={commandInputRef}
                  value={commandInput}
                  onChange={(event) => onCommandInputChange(event.target.value)}
                  placeholder={commandTemplate ? commandTemplate.template : 'Type a command payload'}
                  rows={3}
                />
                <div className="toolbar-row runtime-terminal-compose-toolbar">
                  <label className="checkbox-row">
                    <input type="checkbox" checked={appendNewline} onChange={(event) => onAppendNewlineChange(event.target.checked)} />
                    <span>Append newline</span>
                  </label>
                  <button className="primary-button" onClick={selectedCommandItem ? sendSelectedCommand : onSend}>
                    Send
                  </button>
                </div>
              </div>
            </section>
          </div>
        </section>
      </div>
    </section>
  )
}

function RuntimeTrafficRow({ entry, mode = 'ascii' }: { entry: ConsoleEntry; mode?: ConsoleDisplayMode }) {
  const content = formatEntry(entry, mode)

  return (
    <article className="runtime-entry-row">
      <div className="console-line-meta">
        <span className="console-badge">{entry.direction.toUpperCase()}</span>
        <span className="console-time">{formatTime(entry.timestampMs)}</span>
        {entry.parser?.parserName ? <span className="metric-chip">{entry.parser.parserName}</span> : null}
        <span className="metric-chip">{entry.raw.length} B</span>
      </div>
      <div className="runtime-entry-body">
        <strong>{content.summary}</strong>
        <code>{content.content}</code>
      </div>
    </article>
  )
}

function RuntimePacketGroupRow({ group }: { group: PacketGroup }) {
  const latest = group.entries[group.entries.length - 1]!
  const oldest = group.entries[0]!
  const renderedEntries = [...group.entries].reverse()
  const changeCount = Math.max(0, group.entries.length - 1)

  return (
    <article className="runtime-entry-row runtime-terminal-packet-group runtime-terminal-packet-group-condensed">
      <div className="runtime-terminal-packet-group-header">
        <div className="console-line-meta">
          <span className="console-badge">RX</span>
          <span className="console-time">{formatTime(latest.timestampMs)}</span>
          {latest.parser?.parserName ? <span className="metric-chip">{latest.parser.parserName}</span> : null}
          <span className="metric-chip">{group.shapeLabel}</span>
        </div>
        <div className="runtime-terminal-packet-group-meta">
          <span className="metric-chip tone-success">x{group.entries.length}</span>
          <span className="metric-chip">Δ{changeCount}</span>
          <span className="metric-chip">{oldest.timestampMs === latest.timestampMs ? 'steady' : `${formatTime(oldest.timestampMs)} → ${formatTime(latest.timestampMs)}`}</span>
        </div>
      </div>
      <div className="runtime-terminal-packet-group-body">
        {renderedEntries.map((entry, index) => {
          const previous = renderedEntries[index + 1] ?? null
          return <RuntimePacketLine key={`${group.key}-${index}-${entry.id}`} entry={entry} previous={previous} condensed={index !== renderedEntries.length - 1} />
        })}
      </div>
    </article>
  )
}

function RuntimePacketLine({ entry, previous, condensed }: { entry: ConsoleEntry; previous: ConsoleEntry | null; condensed: boolean }) {
  const tokens = useMemo(() => tokenizePacketText(entry.text), [entry.text])
  const previousTokens = useMemo(() => (previous ? tokenizePacketText(previous.text) : []), [previous])
  const renderedTokens = useMemo(() => renderTokenDiff(tokens, previousTokens), [previousTokens, tokens])
  const changedTokens = renderedTokens.filter((token) => token.isNumeric && token.changed)

  return (
    <div className={`runtime-terminal-packet-line ${condensed ? 'runtime-terminal-packet-line-condensed' : 'runtime-terminal-packet-line-full'}`}>
      <div className="runtime-terminal-packet-line-meta">
        <span className="console-time">{formatTime(entry.timestampMs)}</span>
        <span className="metric-chip">{entry.raw.length} B</span>
        {condensed ? <span className="metric-chip tone-success">condensed</span> : null}
      </div>
      {condensed ? (
        <code className="runtime-terminal-packet-delta">
          {changedTokens.length > 0 ? changedTokens.map((token, index) => (
            <span key={`${entry.id}-delta-${index}`} className={token.className}>
              {token.value}
            </span>
          )) : <span className="runtime-terminal-packet-values-empty">No numeric deltas</span>}
        </code>
      ) : (
        <>
          <code className="runtime-terminal-packet-frame">
            {renderedTokens.map((token, index) => (
              <span key={`${entry.id}-${index}`} className={token.className}>
                {token.value}
              </span>
            ))}
          </code>
          <div className="runtime-terminal-packet-raw">ASCII · {formatBytes(entry.raw)}</div>
        </>
      )}
    </div>
  )
}

function buildPacketGroups(entries: ConsoleEntry[], mode: ConsoleDisplayMode): PacketGroup[] {
  if (entries.length === 0) {
    return []
  }

  const groups: PacketGroup[] = []
  let groupIndex = 0

  for (const entry of entries) {
    const tokenized = tokenizePacketText(entry.text)
    const shapeLabel = describePacketShape(entry, mode, tokenized)
    const lastGroup = groups[groups.length - 1]
    if (lastGroup && lastGroup.shapeLabel === shapeLabel) {
      lastGroup.entries.push(entry)
      continue
    }

    groups.push({ key: `${shapeLabel}#${groupIndex}`, entries: [entry], shapeLabel, shapeTokens: tokenized })
    groupIndex += 1
  }

  return groups
}

function describePacketShape(entry: ConsoleEntry, mode: ConsoleDisplayMode, tokens = tokenizePacketText(entry.text)) {
  if (mode !== 'ascii') {
    return `${entry.parser?.parserName ?? 'raw'} · ${entry.raw.length} B`
  }

  return `${entry.parser?.parserName ?? 'raw'} · ${tokens.map((token) => (token.kind === 'number' ? '#' : token.value)).join('')}`
}

function tokenizePacketText(text: string) {
  const tokens: PacketToken[] = []
  const pattern = /(-?\d+(?:\.\d+)?(?:e[+-]?\d+)?)/gi
  let lastIndex = 0

  for (const match of text.matchAll(pattern)) {
    if (match.index === undefined) continue
    if (match.index > lastIndex) {
      tokens.push({ kind: 'text', value: text.slice(lastIndex, match.index) })
    }

    const value = match[0]
    tokens.push({ kind: 'number', value, numericValue: Number(value) })
    lastIndex = match.index + value.length
  }

  if (lastIndex < text.length) {
    tokens.push({ kind: 'text', value: text.slice(lastIndex) })
  }

  return tokens.length > 0 ? tokens : ([{ kind: 'text', value: text || '[empty payload]' }] as PacketToken[])
}

function renderTokenDiff(tokens: PacketToken[], previousTokens: PacketToken[]) {
  return tokens.map((token, index) => {
    if (token.kind !== 'number') {
      return { value: token.value, className: 'runtime-terminal-token runtime-terminal-token--text', isNumeric: false, changed: false }
    }

    const previousToken = previousTokens[index]
    if (!previousToken || previousToken.kind !== 'number') {
      return { value: token.value, className: 'runtime-terminal-token runtime-terminal-token--new runtime-terminal-token--up', isNumeric: true, changed: true }
    }

    if (previousToken.value === token.value) {
      return { value: token.value, className: 'runtime-terminal-token runtime-terminal-token--steady', isNumeric: true, changed: false }
    }

    const nextClassName = token.numericValue !== undefined && previousToken.numericValue !== undefined && token.numericValue >= previousToken.numericValue
      ? 'runtime-terminal-token runtime-terminal-token--up'
      : 'runtime-terminal-token runtime-terminal-token--down'

    return { value: token.value, className: nextClassName, isNumeric: true, changed: true }
  })
}
