import { useEffect, useRef } from 'react'
import { useAppStore } from '../store/appStore'

export function ConsolePanel() {
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const send = useAppStore((state) => state.send)
  const scrollRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [consoleEntries])

  return (
    <section className="panel console-panel">
      <div className="console-stream" ref={scrollRef}>
        {consoleEntries.map((entry) => (
          <div key={entry.id} className={`console-line ${entry.direction}`}>
            <span className="console-badge">{entry.direction.toUpperCase()}</span>
            <span className="console-time">{formatTime(entry.timestampMs)}</span>
            <code>{entry.text}</code>
          </div>
        ))}
      </div>
      <div className="console-compose">
        <textarea
          value={commandInput}
          onChange={(event) => setCommandInput(event.target.value)}
          placeholder="Type payload, for example: motor_pwm:120"
          rows={3}
        />
        <div className="toolbar-row">
          <label className="checkbox-row">
            <input type="checkbox" checked={appendNewline} onChange={(event) => setAppendNewline(event.target.checked)} />
            <span>Append newline</span>
          </label>
          <button className="primary-button" onClick={() => void send()}>
            Send
          </button>
        </div>
      </div>
    </section>
  )
}

function formatTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleTimeString()
}
