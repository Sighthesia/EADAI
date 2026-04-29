import type {
  AppStatus,
  SerialBusEvent,
  SessionSnapshot,
  UiProtocolHandshakeEvent,
  UiProtocolSnapshot,
} from '../types'
import { BMI088_PROTOCOL_NAME, PROTOCOL_TIMELINE_LIMIT } from './constants'

export const defaultProtocolSnapshot = (): UiProtocolSnapshot => ({
  active: false,
  parserName: BMI088_PROTOCOL_NAME,
  transportLabel: 'BMI088 UART4',
  baudRate: 115200,
  phase: 'stopped',
  identity: null,
  schema: null,
  lastPacketKind: null,
  lastPacketRawFrame: null,
  lastSchemaAtMs: null,
  lastSampleAtMs: null,
  lastHandshakeAtMs: null,
  timeline: [],
})

export const ingestProtocolConnection = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'connection' }>,
): UiProtocolSnapshot => {
  const nextPhase =
    event.connection.state === 'connected'
      ? 'awaitingIdentity'
      : event.connection.state === 'stopped'
        ? 'stopped'
        : protocol.phase

  const timeline = appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: event.connection.state === 'stopped' ? 'STOP' : 'REQ_IDENTITY',
    note: connectionMessage(event),
    parserStatus: 'unparsed',
  })

  return {
    ...protocol,
    active: event.connection.state !== 'stopped',
    transportLabel: event.source.transport === 'fake' ? 'Fake BMI088 stream' : 'BMI088 UART4',
    baudRate: event.source.baudRate,
    phase: nextPhase,
    timeline,
    lastHandshakeAtMs: event.timestampMs,
  }
}

export const ingestProtocolLine = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'line' }>,
): UiProtocolSnapshot => {
  const parserName = event.parser.parserName ?? protocol.parserName
  const command = protocolCommandFromLine(event.line.text)
  const isHandshake = parserName === 'bmi088_command' || command !== null

  if (!isHandshake) {
    return protocol
  }

  const nextPhase =
    command === 'ACK'
      ? 'awaitingStart'
      : command === 'START'
        ? 'streaming'
        : command === 'STOP'
          ? 'stopped'
          : command === 'REQ_IDENTITY'
            ? 'awaitingIdentity'
            : command === 'REQ_SCHEMA'
              ? 'awaitingSchema'
              : protocol.phase

  return {
    ...protocol,
    active: true,
    parserName,
    phase: nextPhase,
    lastPacketKind: 'command' as const,
    lastPacketRawFrame: event.line.raw,
    lastHandshakeAtMs: event.timestampMs,
    timeline: appendProtocolTimeline(protocol.timeline, {
      timestampMs: event.timestampMs,
      direction: event.line.direction,
      command: command ?? 'REQ_SCHEMA',
      note: protocolCommandNote(command ?? event.line.text),
      parserStatus: event.parser.status,
    }),
  }
}

export const ingestProtocolIdentity = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'telemetryIdentity' }>,
): UiProtocolSnapshot => ({
  ...protocol,
  active: true,
  parserName: event.parser.parserName ?? protocol.parserName,
  phase: protocol.schema ? protocol.phase : 'awaitingSchema',
  identity: event.identity,
  lastPacketKind: 'command' as const,
  lastPacketRawFrame: event.rawFrame,
  lastHandshakeAtMs: event.timestampMs,
  timeline: appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: 'IDENTITY',
    note: `${event.identity.deviceName} · ${event.identity.boardName} · ${event.identity.firmwareVersion}`,
    parserStatus: event.parser.status,
  }),
})

export const ingestProtocolSchema = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'telemetrySchema' }>,
): UiProtocolSnapshot => ({
  ...protocol,
  active: true,
  parserName: event.parser.parserName ?? protocol.parserName,
  phase: 'awaitingAck' as const,
  schema: {
    rateHz: event.schema.rateHz,
    sampleLen: event.schema.sampleLen,
    fields: event.schema.fields,
  },
  lastPacketKind: 'schema' as const,
  lastPacketRawFrame: event.rawFrame,
  lastSchemaAtMs: event.timestampMs,
  lastHandshakeAtMs: event.timestampMs,
  timeline: appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: 'SCHEMA',
    note: `Schema ${event.schema.fields.length} fields @ ${event.schema.rateHz} Hz`,
    parserStatus: event.parser.status,
  }),
})

export const ingestProtocolSample = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'telemetrySample' }>,
): UiProtocolSnapshot => ({
  ...protocol,
  active: true,
  parserName: event.parser.parserName ?? protocol.parserName,
  phase: 'streaming' as const,
  lastPacketKind: 'sample' as const,
  lastPacketRawFrame: event.rawFrame,
  lastSampleAtMs: event.timestampMs,
  lastHandshakeAtMs: event.timestampMs,
  timeline: appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: 'SAMPLE',
    note: `Sample ${event.sample.fields.length} fields`,
    parserStatus: event.parser.status,
  }),
})

export const ingestProtocolShellOutput = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'shellOutput' }>,
): UiProtocolSnapshot => ({
  ...protocol,
  active: true,
  parserName: event.parser.parserName ?? protocol.parserName,
  lastPacketKind: 'command' as const,
  lastPacketRawFrame: event.line.raw,
  lastHandshakeAtMs: event.timestampMs,
  timeline: appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: 'SHELL_OUTPUT',
    note: event.line.text,
    parserStatus: event.parser.status,
  }),
})

const appendProtocolTimeline = (timeline: UiProtocolHandshakeEvent[], next: UiProtocolHandshakeEvent) =>
  [...timeline, next].slice(-PROTOCOL_TIMELINE_LIMIT)

const protocolCommandFromLine = (text: string): UiProtocolHandshakeEvent['command'] | null => {
  const normalized = text.trim().toUpperCase()
  if (
    normalized === 'ACK' ||
    normalized === 'START' ||
    normalized === 'STOP' ||
    normalized === 'REQ_SCHEMA' ||
    normalized === 'REQ_TUNING' ||
    normalized === 'SET_TUNING' ||
    normalized === 'SHELL_EXEC' ||
    normalized === 'SHELL_OUTPUT'
  ) {
    return normalized
  }
  if (normalized === 'REQ_IDENTITY') {
    return normalized
  }
  return null
}

const protocolCommandNote = (command: string) => {
  switch (command) {
    case 'ACK':
      return 'Host acknowledgement'
    case 'START':
      return 'Start streaming'
    case 'STOP':
      return 'Stop streaming'
    case 'REQ_SCHEMA':
      return 'Request schema'
    case 'REQ_IDENTITY':
      return 'Request identity'
    default:
      return command
  }
}

export const connectionMessage = (event: Extract<SerialBusEvent, { kind: 'connection' }>) => {
  const sourceLabel = event.source.transport === 'fake' ? 'fake stream' : 'serial'
  const label = `${event.source.port} @ ${event.source.baudRate} (${sourceLabel})`

  switch (event.connection.state) {
    case 'connected':
      return `Connected to ${label}.`
    case 'connecting':
      return `Connecting to ${label} (attempt ${event.connection.attempt}).`
    case 'waitingRetry':
      return event.connection.reason
        ? `Retrying ${label}: ${event.connection.reason}`
        : `Retrying ${label}.`
    case 'stopped':
      return `Stopped ${label}.`
    default:
      return `Session is ${event.connection.state}.`
  }
}

export const connectionTone = (state: SessionSnapshot['connectionState']): AppStatus['tone'] => {
  switch (state) {
    case 'connected':
      return 'success'
    case 'waitingRetry':
      return 'warning'
    case 'stopped':
      return 'neutral'
    default:
      return 'neutral'
  }
}
