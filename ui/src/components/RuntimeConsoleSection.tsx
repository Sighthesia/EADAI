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
  return (
    <section className="runtime-section runtime-console-section">
      <RuntimeSectionHeader title="Serial tools" />
      <div className="runtime-console-strip">
        <span className="metric-chip">{consoleDisplayMode.toUpperCase()}</span>
        <span className="metric-chip">{consoleEntries.length} lines</span>
        <span className="metric-chip">{appendNewline ? 'newline on' : 'newline off'}</span>
      </div>

      <section className="runtime-section-card">
        <div className="protocol-schema-header">
          <strong>Console tools</strong>
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
        </div>
        <div className="runtime-entry-list runtime-console-entry-list">
          {consoleEntries.slice(-8).reverse().length > 0
            ? consoleEntries.slice(-8).reverse().map((entry) => <RuntimeTrafficRow key={entry.id} entry={entry} mode={consoleDisplayMode} />)
            : <div className="protocol-empty">Awaiting serial traffic.</div>}
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
