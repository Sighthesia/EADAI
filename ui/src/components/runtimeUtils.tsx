import type { Bmi088HostCommand, ConsoleEntry, UiProtocolHandshakeEvent, UiProtocolHandshakePhase, UiTriggerPayload, VariableDefinition, VariableDefinitionVisibility, VariableEntry, VariableSourceKind } from '../types'
import type { useAppStore } from '../store/appStore'

export type HookStatus = {
  label: string
  detail: string
}

export type SurfaceStatus = {
  label: string
  detail: string
}

export type VariableDefinitionDeviceGroup = {
  deviceRef: string
  label: string
  detail: string
  definitions: VariableDefinition[]
}

export type VariableDefinitionSurface = 'runtime' | 'variables'

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

export function buildScriptSurfaceStatus(script: string, exampleCount: number, extractionCount: number): SurfaceStatus {
  if (script.trim().length === 0) return { label: 'Definition missing', detail: 'Add the shared protocol definition before authoring new authoring lanes.' }
  if (exampleCount > 0 || extractionCount > 0) {
    return {
      label: 'Definition ready',
      detail: `${exampleCount} definition examples and ${extractionCount} extraction targets are available.`,
    }
  }
  return { label: 'Definition ready', detail: 'The protocol definition is loaded and ready for new authoring lanes.' }
}

export function buildRuntimeActivityStatus(triggerCount: number): SurfaceStatus {
  if (triggerCount > 0) return { label: 'Runtime firing', detail: `${triggerCount} recent trigger events captured.` }
  return { label: 'Idle', detail: 'Waiting for trigger activity.' }
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

export function collectRecentVariableDefinitions(definitions: VariableDefinition[]) {
  return [...definitions]
    .sort((left, right) => right.updatedAtMs - left.updatedAtMs)
    .slice(0, 5)
}

export function describeVariableSourceKind(sourceKind: VariableSourceKind) {
  if (sourceKind === 'protocol-text') return 'Protocol text'
  return 'Telemetry sample'
}

export function describeVariableVisibility(visibility: VariableDefinitionVisibility) {
  if (visibility === 'both') return 'Runtime + Variables'
  if (visibility === 'runtime') return 'Runtime only'
  if (visibility === 'variables') return 'Variables only'
  return 'Hidden'
}

export function isVariableDefinitionVisibleInSurface(definition: VariableDefinition, surface: VariableDefinitionSurface) {
  return definition.visibility === 'both' || definition.visibility === surface
}

export function filterVariableDefinitionsBySurface(definitions: VariableDefinition[], surface: VariableDefinitionSurface) {
  return definitions.filter((definition) => isVariableDefinitionVisibleInSurface(definition, surface))
}

export function countVariableDefinitionsByVisibility(definitions: VariableDefinition[]) {
  return definitions.reduce(
    (counts, definition) => {
      counts[definition.visibility] += 1
      return counts
    },
    { both: 0, runtime: 0, variables: 0, hidden: 0 } as Record<VariableDefinitionVisibility, number>,
  )
}

export function countVariableDefinitionsBySourceKind(definitions: VariableDefinition[]) {
  return definitions.reduce(
    (counts, definition) => {
      counts[definition.sourceKind] += 1
      return counts
    },
    { 'protocol-text': 0, 'telemetry-sample': 0 } as Record<VariableSourceKind, number>,
  )
}

export function groupVariableDefinitionsByDevice(definitions: VariableDefinition[], activeDeviceRef?: string | null) {
  const groups = new Map<string, VariableDefinitionDeviceGroup>()

  for (const definition of definitions) {
    const deviceRef = definition.deviceRef ?? 'unassigned'
    const existing = groups.get(deviceRef)
    if (existing) {
      existing.definitions.push(definition)
      continue
    }

    groups.set(deviceRef, {
      deviceRef,
      label: deviceRef === 'unassigned' ? 'Ungrouped runtime variables' : deviceRef,
      detail: deviceRef === 'unassigned' ? 'Variable definitions without a stable device ref yet' : 'Variable definitions tied to this runtime device',
      definitions: [definition],
    })
  }

  return [...groups.values()].sort((left, right) => {
    const leftPriority = left.deviceRef === (activeDeviceRef ?? '') ? 0 : left.deviceRef === 'unassigned' ? 2 : 1
    const rightPriority = right.deviceRef === (activeDeviceRef ?? '') ? 0 : right.deviceRef === 'unassigned' ? 2 : 1
    if (leftPriority !== rightPriority) return leftPriority - rightPriority
    return left.label.localeCompare(right.label, undefined, { numeric: true, sensitivity: 'base' })
  })
}

export function findVariableDefinition(definitions: VariableDefinition[], name: string) {
  return definitions.find((definition) => definition.name === name) ?? null
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
    case 'awaitingIdentity':
      return 'REQ_IDENTITY'
    case 'awaitingSchema':
      return 'REQ_SCHEMA'
    case 'awaitingAck':
      return 'ACK'
    case 'awaitingStart':
      return 'START'
    case 'streaming':
      return 'STOP'
    case 'stopped':
      return 'REQ_IDENTITY'
    default:
      return null
  }
}

export function isCommandActiveForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  return recommendedCommandForPhase(phase) === command
}

export function isCommandDisabledForPhase(phase: UiProtocolHandshakePhase, command: Bmi088HostCommand) {
  if (phase === 'awaitingIdentity') return command === 'ACK' || command === 'START' || command === 'STOP'
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
