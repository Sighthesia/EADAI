import type { Bmi088HostCommand, ConsoleEntry } from '../types'
import { formatBytes, formatEntry, formatTime, RuntimeSectionHeader } from './runtimeUtils'

export function RuntimeConsoleSection({
  runtimeCommands,
  commandInput,
  consoleDisplayMode,
  consoleEntries,
  appendNewline,
  onCommandInputChange,
  onAppendNewlineChange,
  onDisplayModeChange,
  onSend,
  onSendCommand,
}: {
  runtimeCommands: { command: Bmi088HostCommand; description: string }[]
  commandInput: string
  consoleDisplayMode: 'text' | 'hex' | 'binary'
  consoleEntries: ConsoleEntry[]
  appendNewline: boolean
  onCommandInputChange: (value: string) => void
  onAppendNewlineChange: (value: boolean) => void
  onDisplayModeChange: (value: 'text' | 'hex' | 'binary') => void
  onSend: () => void
  onSendCommand: (command: Bmi088HostCommand) => void
}) {
  const recentEntries = consoleEntries.slice(-8).reverse()

  return (
    <section className="runtime-section runtime-console-section">
      <RuntimeSectionHeader title="Raw serial inspector" description="Console tools and recent bytes for quick traffic inspection" />
      <div className="runtime-summary-grid runtime-console-summary-grid">
        <article className="runtime-card">
          <span className="mcp-label">Traffic</span>
          <strong>{consoleEntries.length} lines</strong>
          <small>{recentEntries[0] ? summarizeLatestTraffic(recentEntries[0]) : 'Waiting for the first line'}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Display</span>
          <strong>{consoleDisplayMode.toUpperCase()}</strong>
          <small>Toggle how payload bytes are rendered</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Compose</span>
          <strong>{appendNewline ? 'Newline on' : 'Newline off'}</strong>
          <small>Send typed payloads directly to the serial line</small>
        </article>
      </div>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Console tools</strong>
          <small>Keep raw traffic visible while changing display mode or sending payloads</small>
        </div>
        <div className="toolbar-row runtime-console-toolbar">
          <div className="console-display-switch" role="group" aria-label="Console display mode">
            {(['text', 'hex', 'binary'] as const).map((mode) => (
              <button key={mode} type="button" className={`metric-display-button ${consoleDisplayMode === mode ? 'active' : ''}`} onClick={() => onDisplayModeChange(mode)}>
                {mode.toUpperCase()}
              </button>
            ))}
          </div>
          <div className="console-command-strip" role="group" aria-label="BMI088 commands">
            {runtimeCommands.map((item) => (
              <button key={item.command} type="button" className="ghost-button" onClick={() => onSendCommand(item.command)} title={item.description}>
                {item.command}
              </button>
            ))}
          </div>
        </div>
        <div className="console-compose runtime-console-compose">
          <textarea value={commandInput} onChange={(event) => onCommandInputChange(event.target.value)} placeholder="Type payload, for example: motor_pwm:120" rows={3} />
          <div className="toolbar-row">
            <label className="checkbox-row">
              <input type="checkbox" checked={appendNewline} onChange={(event) => onAppendNewlineChange(event.target.checked)} />
              <span>Append newline</span>
            </label>
            <button className="primary-button" onClick={onSend}>
              Send
            </button>
          </div>
        </div>
      </section>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Latest traffic</strong>
          <small>Raw serial entries and parser state</small>
        </div>
        <div className="runtime-entry-list runtime-console-entry-list">
          {recentEntries.length > 0 ? recentEntries.map((entry) => <RuntimeTrafficRow key={entry.id} entry={entry} mode={consoleDisplayMode} />) : <div className="protocol-empty">Awaiting serial traffic.</div>}
        </div>
      </section>
    </section>
  )
}

function RuntimeTrafficRow({ entry, mode = 'text' }: { entry: ConsoleEntry; mode?: 'text' | 'hex' | 'binary' }) {
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
        <code>{formatBytes(entry.raw)}</code>
        {content.content !== content.summary ? <small>{content.content}</small> : null}
      </div>
    </article>
  )
}

function summarizeLatestTraffic(entry: ConsoleEntry | null) {
  if (!entry) return 'Waiting for the first line'
  const parserLabel = entry.parser?.parserName ? `${entry.parser.parserName} · ` : ''
  return `${parserLabel}${entry.direction.toUpperCase()} ${entry.raw.length} B`
}
