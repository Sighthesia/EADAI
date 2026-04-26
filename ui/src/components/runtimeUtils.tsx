import type { Bmi088HostCommand, ConsoleEntry, UiProtocolHandshakeEvent, UiProtocolHandshakePhase, UiTriggerPayload, VariableEntry } from '../types'
import type { useAppStore } from '../store/appStore'

export type HookStatus = {
  label: string
  detail: string
}

export function RuntimeSectionHeader({ title, description }: { title: string; description: string }) {
  return (
    <div className="runtime-section-header">
      <div>
        <span className="mcp-label">Runtime section</span>
        <h3>{title}</h3>
      </div>
      <small>{description}</small>
    </div>
  )
}

export function RuntimeFlowStep({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <article className="runtime-flow-step">
      <span className="mcp-label">{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  )
}

export function summarizeTraffic(entries: ConsoleEntry[]) {
  return entries.length === 0 ? 'No traffic yet' : `${entries.length} serial lines`
}

export function summarizeLatestTraffic(entry: ConsoleEntry | null) {
  if (!entry) return 'Waiting for the first line'
  const parserLabel = entry.parser?.parserName ? `${entry.parser.parserName} · ` : ''
  return `${parserLabel}${entry.direction.toUpperCase()} ${entry.raw.length} B`
}

export function buildHookStatus(script: string, exampleCount: number, triggerCount: number): HookStatus {
  if (triggerCount > 0) return { label: 'Runtime firing', detail: `${triggerCount} recent trigger events captured.` }
  if (exampleCount > 0 && script.trim().length > 0) return { label: 'Hook ready', detail: `${exampleCount} examples and the shared script are available.` }
  return { label: 'Hook idle', detail: 'Waiting for script examples or trigger activity.' }
}

export function formatTime(timestampMs: number | null) {
  return timestampMs === null ? '-' : new Date(timestampMs).toLocaleTimeString()
}

export function formatBytes(bytes: number[]) {
  return bytes.map((byte) => byte.toString(16).padStart(2, '0').toUpperCase()).join(' ')
}

export function formatEntry(entry: ConsoleEntry, mode: 'text' | 'hex' | 'binary') {
  if (mode === 'text') return { summary: entry.text || '[empty payload]', content: entry.text || '[empty payload]' }
  if (mode === 'hex') return { summary: `${entry.raw.length} bytes`, content: formatBytes(entry.raw) }
  return { summary: `${entry.raw.length} bytes`, content: entry.raw.map((byte) => byte.toString(2).padStart(8, '0')).join(' ') }
}

export function collectRecentTriggers(variables: Record<string, VariableEntry>) {
  return Object.values(variables)
    .flatMap((variable) => (variable.latestTrigger ? [variable.latestTrigger] : []))
    .sort((left, right) => right.firedAtMs - left.firedAtMs)
    .slice(0, 5)
}

export function protocolCommandStates(phase: UiProtocolHandshakePhase, commands: ReturnType<typeof useAppStore.getState>['runtimeCatalog']['commands']) {
  const recommended = recommendedCommandForPhase(phase)

  return commands.map((item) => ({
    command: item.command,
    recommended: item.command === recommended,
    disabled: isCommandDisabledForPhase(phase, item.command),
    active: isCommandActiveForPhase(phase, item.command),
    reason: commandReasonForPhase(phase, item.command),
  }))
}

export function recommendedCommandForPhase(phase: UiProtocolHandshakePhase): Bmi088HostCommand | null {
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

export function isCommandActiveForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  return recommendedCommandForPhase(phase) === command
}

export function isCommandDisabledForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  if (phase === 'awaitingSchema') return command === 'ACK' || command === 'START'
  if (phase === 'awaitingAck') return command === 'START'
  if (phase === 'stopped') return command === 'ACK'
  return false
}

export function commandReasonForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  const recommended = recommendedCommandForPhase(phase)
  if (command === recommended) return `Recommended while phase is ${phase}.`
  if (isCommandDisabledForPhase(phase, command)) return `Usually not valid while phase is ${phase}.`
  return `Available while phase is ${phase}.`
}

export function RuntimeTimelineRow({ event }: { event: UiProtocolHandshakeEvent }) {
  return (
    <article className={`protocol-timeline-item direction-${event.direction}`}>
      <div className="protocol-timeline-row">
        <span className="metric-chip">{event.direction.toUpperCase()}</span>
        <strong>{event.command}</strong>
        <small>{formatTime(event.timestampMs)}</small>
        <span className={`metric-chip tone-${event.parserStatus}`}>{event.parserStatus}</span>
      </div>
      <p>{event.note}</p>
    </article>
  )
}

export function RuntimeTriggerRow({ trigger }: { trigger: UiTriggerPayload }) {
  return (
    <article className="runtime-trigger-item">
      <div className="runtime-trigger-row">
        <span className={`metric-chip tone-${trigger.severity}`}>{trigger.severity}</span>
        <strong>{trigger.ruleId}</strong>
        <small>{formatTime(trigger.firedAtMs)}</small>
      </div>
      <p>
        {trigger.channelId}: {trigger.reason}
      </p>
    </article>
  )
}
