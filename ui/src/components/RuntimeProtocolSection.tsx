import type { Bmi088HostCommand, UiProtocolHandshakeEvent, UiProtocolHandshakePhase, UiRuntimeCommandCatalogItem } from '../types'
import { formatBytes, formatTime, RuntimeSectionHeader, RuntimeTimelineRow, protocolCommandStates } from './runtimeUtils'

export function RuntimeProtocolSection({
  protocol,
  recentTimeline,
  runtimeCommands,
  onSendCommand,
}: {
  protocol: {
    transportLabel: string
    baudRate: number
    phase: UiProtocolHandshakePhase
    active: boolean
    lastPacketKind?: string | null
    lastHandshakeAtMs?: number | null
    identity?: {
      deviceName: string
      boardName: string
      firmwareVersion: string
      protocolName: string
      protocolVersion: string
      transportName: string
      sampleRateHz: number
      schemaFieldCount: number
      samplePayloadLen: number
      featureFlags: number
      baudRate: number
      protocolMinorVersion: number
    } | null
    schema?: { rateHz: number; sampleLen: number; fields: { name: string; unit: string; scaleQ: number }[] } | null
    timeline: UiProtocolHandshakeEvent[]
    lastPacketRawFrame?: number[] | null
  }
  recentTimeline: UiProtocolHandshakeEvent[]
  runtimeCommands: UiRuntimeCommandCatalogItem[]
  onSendCommand: (command: Bmi088HostCommand, payload?: string | null) => void
}) {
  const commandStates = protocolCommandStates(protocol.phase, runtimeCommands)

  return (
    <section className="runtime-section runtime-protocol-section">
      <RuntimeSectionHeader title="Protocol diagnostics" />
      <div className="runtime-summary-grid runtime-protocol-summary-grid">
        <article className="runtime-card">
          <span className="mcp-label">Transport</span>
          <strong>{protocol.transportLabel}</strong>
          <small>{protocol.baudRate} baud</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Handshake</span>
          <strong>{protocol.phase}</strong>
          <small>{protocol.active ? 'Active' : 'Idle'}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Last packet</span>
          <strong>{protocol.lastPacketKind ?? '-'}</strong>
          <small>{formatTime(protocol.lastHandshakeAtMs ?? null)}</small>
        </article>
      </div>

      <details className="runtime-disclosure">
        <summary>
          <strong>Handshake controls and timeline</strong>
        </summary>
        <div className="runtime-disclosure-body">
          <section className="runtime-section-card">
            <div className="protocol-schema-header">
              <strong>Commands</strong>
            </div>
            <div className="protocol-command-row runtime-protocol-command-row" role="group" aria-label="BMI088 protocol commands">
              {commandStates.map((item) => (
                 <button key={item.command} type="button" className={`ghost-button protocol-command-button ${item.recommended ? 'recommended' : ''}`} disabled={item.disabled} onClick={() => onSendCommand(item.command)} title={item.reason}>
                  <span>{item.command}</span>
                  {item.recommended ? <small>recommended</small> : null}
                  {!item.recommended && !item.disabled && item.active ? <small>active</small> : null}
                </button>
              ))}
            </div>
          </section>

          <section className="runtime-section-card">
            <div className="protocol-schema-header">
              <strong>Handshake timeline</strong>
            </div>
            <div className="protocol-timeline runtime-timeline">
              {recentTimeline.length > 0 ? recentTimeline.map((event) => <RuntimeTimelineRow key={`${event.timestampMs}-${event.command}-${event.direction}`} event={event} />) : <div className="protocol-empty">Awaiting BMI088 handshake.</div>}
            </div>
          </section>
        </div>
      </details>

      {protocol.identity ? (
        <details className="runtime-disclosure">
          <summary>
            <strong>Identity snapshot</strong>
          </summary>
          <div className="runtime-disclosure-body">
            <section className="runtime-section-card">
              <div className="protocol-field-grid">
                <div className="protocol-field-pill">
                  <strong>Firmware</strong>
                  <small>{protocol.identity.firmwareVersion}</small>
                </div>
                <div className="protocol-field-pill">
                  <strong>Protocol</strong>
                  <small>
                    {protocol.identity.protocolName} {protocol.identity.protocolVersion}.{protocol.identity.protocolMinorVersion}
                  </small>
                </div>
                <div className="protocol-field-pill">
                  <strong>Transport</strong>
                  <small>
                    {protocol.identity.transportName} · {protocol.identity.baudRate} baud
                  </small>
                </div>
                <div className="protocol-field-pill">
                  <strong>Telemetry</strong>
                  <small>
                    {protocol.identity.sampleRateHz} Hz · {protocol.identity.schemaFieldCount} fields · {protocol.identity.samplePayloadLen} bytes
                  </small>
                </div>
                <div className="protocol-field-pill">
                  <strong>Features</strong>
                  <small>{formatFeatureFlags(protocol.identity.featureFlags)}</small>
                </div>
              </div>
            </section>
          </div>
        </details>
      ) : null}

      {protocol.schema ? (
        <details className="runtime-disclosure">
          <summary>
            <strong>Schema reference</strong>
          </summary>
          <div className="runtime-disclosure-body">
            <section className="runtime-section-card">
              <div className="protocol-field-grid">
                {protocol.schema.fields.map((field, index) => (
                  <div key={`${field.name}-${index}`} className="protocol-field-pill">
                    <strong>
                      {index + 1}. {field.name}
                    </strong>
                    <small>
                      {field.unit} · q{field.scaleQ}
                    </small>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </details>
      ) : null}

      {protocol.lastPacketRawFrame?.length ? (
        <details className="runtime-disclosure">
          <summary>
            <strong>Raw frame</strong>
          </summary>
          <div className="runtime-disclosure-body">
            <section className="runtime-section-card">
              <pre>{formatBytes(protocol.lastPacketRawFrame)}</pre>
            </section>
          </div>
        </details>
      ) : null}
    </section>
  )
}

function formatFeatureFlags(flags: number) {
  const features = [
    [0x0001, 'binary commands'],
    [0x0002, 'shell mode'],
    [0x0004, 'identity'],
    [0x0008, 'schema'],
    [0x0010, 'streaming'],
  ] as const

  const enabled = features.filter(([bit]) => (flags & bit) !== 0).map(([, label]) => label)
  return enabled.length > 0 ? `${enabled.join(' · ')} (0x${flags.toString(16).toUpperCase().padStart(4, '0')})` : `0x${flags.toString(16).toUpperCase().padStart(4, '0')}`
}
