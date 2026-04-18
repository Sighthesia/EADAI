import { useAppStore } from '../store/appStore'
import type { Bmi088HostCommand, UiProtocolHandshakePhase } from '../types'

export function ProtocolPanel() {
  const protocol = useAppStore((state) => state.protocol)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)
  const commandStates = protocolCommandStates(protocol.phase)

  return (
    <section className="panel panel-scroll protocol-panel">
      <div className="variables-header">
        <span>BMI088 protocol</span>
        <div className="variables-header-actions">
          <span>{protocol.parserName}</span>
          <span>{protocol.phase}</span>
        </div>
      </div>

      <div className="protocol-summary-grid">
        <article className="protocol-card">
          <span className="mcp-label">Transport</span>
          <strong>{protocol.transportLabel}</strong>
          <small>{protocol.baudRate} baud</small>
        </article>
        <article className="protocol-card">
          <span className="mcp-label">Handshake</span>
          <strong>{protocol.phase}</strong>
          <small>{protocol.active ? 'Active' : 'Idle'}</small>
        </article>
        <article className="protocol-card">
          <span className="mcp-label">Last packet</span>
          <strong>{protocol.lastPacketKind ?? '-'}</strong>
          <small>{formatTime(protocol.lastHandshakeAtMs ?? null)}</small>
        </article>
      </div>

      <section className="protocol-command-card">
        <div className="protocol-schema-header">
          <strong>Commands</strong>
          <small>Send BMI088 request frames</small>
        </div>
        <div className="protocol-command-row" role="group" aria-label="BMI088 protocol commands">
          {commandStates.map((item) => (
            <button
              key={item.command}
              type="button"
              className={`ghost-button protocol-command-button ${item.recommended ? 'recommended' : ''}`}
              disabled={item.disabled}
              onClick={() => void sendBmi088Command(item.command)}
              title={item.reason}
            >
              <span>{item.command}</span>
              {item.recommended ? <small>recommended</small> : null}
              {!item.recommended && !item.disabled && item.active ? <small>active</small> : null}
            </button>
          ))}
        </div>
      </section>

      {protocol.schema ? (
        <section className="protocol-schema-card">
          <div className="protocol-schema-header">
            <strong>Schema</strong>
            <small>{protocol.schema.rateHz} Hz · {protocol.schema.sampleLen} bytes</small>
          </div>
          <div className="protocol-field-grid">
            {protocol.schema.fields.map((field, index) => (
              <div key={`${field.name}-${index}`} className="protocol-field-pill">
                <strong>{index + 1}. {field.name}</strong>
                <small>{field.unit} · q{field.scaleQ}</small>
              </div>
            ))}
          </div>
        </section>
      ) : null}

      <section className="protocol-timeline-card">
        <div className="protocol-schema-header">
          <strong>Handshake timeline</strong>
          <small>{protocol.timeline.length} events</small>
        </div>
        <div className="protocol-timeline">
          {protocol.timeline.length > 0 ? protocol.timeline.map((item) => (
            <article key={`${item.timestampMs}-${item.command}-${item.direction}`} className={`protocol-timeline-item direction-${item.direction}`}>
              <div className="protocol-timeline-row">
                <span className="metric-chip">{item.direction.toUpperCase()}</span>
                <strong>{item.command}</strong>
                <small>{formatTime(item.timestampMs)}</small>
                <span className={`metric-chip tone-${item.parserStatus}`}>{item.parserStatus}</span>
              </div>
              <p>{item.note}</p>
            </article>
          )) : <div className="protocol-empty">Awaiting BMI088 handshake.</div>}
        </div>
      </section>

      {protocol.lastPacketRawFrame?.length ? (
        <section className="protocol-raw-card">
          <div className="protocol-schema-header">
            <strong>Raw frame</strong>
            <small>{protocol.lastPacketRawFrame.length} bytes</small>
          </div>
          <pre>{formatBytes(protocol.lastPacketRawFrame)}</pre>
        </section>
      ) : null}
    </section>
  )
}

const formatTime = (timestampMs: number | null) => (timestampMs === null ? '-' : new Date(timestampMs).toLocaleTimeString())

const formatBytes = (bytes: number[]) => bytes.map((byte) => byte.toString(16).padStart(2, '0').toUpperCase()).join(' ')

type ProtocolCommandState = {
  command: Bmi088HostCommand
  recommended: boolean
  disabled: boolean
  active: boolean
  reason: string
}

function protocolCommandStates(phase: UiProtocolHandshakePhase): ProtocolCommandState[] {
  const recommended = recommendedCommandForPhase(phase)

  return (['ACK', 'START', 'STOP', 'REQ_SCHEMA'] as const).map((command) => ({
    command,
    recommended: command === recommended,
    disabled: isCommandDisabledForPhase(phase, command),
    active: isCommandActiveForPhase(phase, command),
    reason: commandReasonForPhase(phase, command),
  }))
}

function recommendedCommandForPhase(phase: UiProtocolHandshakePhase): Bmi088HostCommand | null {
  switch (phase) {
    case 'awaitingSchema':
      return 'REQ_SCHEMA'
    case 'awaitingAck':
      return 'ACK'
    case 'awaitingStart':
      return 'START'
    case 'streaming':
      return 'STOP'
    case 'stopped':
      return 'REQ_SCHEMA'
    default:
      return null
  }
}

function isCommandActiveForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  return recommendedCommandForPhase(phase) === command
}

function isCommandDisabledForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  if (phase === 'awaitingSchema') {
    return command === 'ACK' || command === 'START'
  }

  if (phase === 'awaitingAck') {
    return command === 'START'
  }

  if (phase === 'stopped') {
    return command === 'ACK'
  }

  return false
}

function commandReasonForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  const recommended = recommendedCommandForPhase(phase)
  if (command === recommended) {
    return `Recommended while phase is ${phase}.`
  }

  if (isCommandDisabledForPhase(phase, command)) {
    return `Usually not valid while phase is ${phase}.`
  }

  return `Available while phase is ${phase}.`
}
