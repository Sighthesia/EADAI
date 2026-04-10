import { create } from 'zustand'
import {
  connectSerial,
  disconnectSerial,
  getSessionSnapshot,
  listSerialPorts,
  sendSerial,
} from '../lib/tauri'
import type {
  AppStatus,
  ConnectRequest,
  ConsoleEntry,
  SerialBusEvent,
  SessionSnapshot,
  VariableEntry,
} from '../types'

const MAX_CONSOLE_ENTRIES = 400
const MAX_SAMPLES_PER_CHANNEL = 480
const CHANNEL_COLORS = ['#4FC3F7', '#C792EA', '#F78C6C', '#A5E075', '#E6C07B', '#82AAFF']

type AppStore = {
  ports: string[]
  session: SessionSnapshot
  config: ConnectRequest
  appendNewline: boolean
  commandInput: string
  consoleEntries: ConsoleEntry[]
  variables: Record<string, VariableEntry>
  selectedChannels: string[]
  status: AppStatus
  setCommandInput: (value: string) => void
  setAppendNewline: (value: boolean) => void
  patchConfig: (value: Partial<ConnectRequest>) => void
  bootstrap: () => Promise<void>
  refreshPorts: () => Promise<void>
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  send: () => Promise<void>
  ingestEvent: (event: SerialBusEvent) => void
  toggleChannel: (channel: string) => void
  colorForChannel: (channel: string) => string
}

const defaultStatus = (): AppStatus => ({
  tone: 'neutral',
  message: 'Ready. Select a serial port and start a session.',
})

export const useAppStore = create<AppStore>((set, get) => ({
  ports: [],
  session: { isRunning: false, connectionState: 'stopped' },
  config: {
    port: '',
    baudRate: 115200,
    retryMs: 1000,
    readTimeoutMs: 50,
  },
  appendNewline: true,
  commandInput: '',
  consoleEntries: [],
  variables: {},
  selectedChannels: [],
  status: defaultStatus(),
  setCommandInput: (value) => set({ commandInput: value }),
  setAppendNewline: (value) => set({ appendNewline: value }),
  patchConfig: (value) => set((state) => ({ config: { ...state.config, ...value } })),
  bootstrap: async () => {
    const [ports, session] = await Promise.all([listSerialPorts(), getSessionSnapshot()])
    set((state) => ({
      ports,
      session,
      config: {
        ...state.config,
        port: session.port ?? ports[0] ?? state.config.port,
        baudRate: session.baudRate ?? state.config.baudRate,
      },
      status: session.isRunning
        ? {
            tone: 'success',
            message: `Session on ${session.port ?? 'serial'} is ${session.connectionState ?? 'running'}.`,
          }
        : defaultStatus(),
    }))
  },
  refreshPorts: async () => {
    const ports = await listSerialPorts()
    set((state) => ({
      ports,
      config: {
        ...state.config,
        port: ports.includes(state.config.port) ? state.config.port : ports[0] ?? '',
      },
      status: {
        tone: 'neutral',
        message: ports.length > 0 ? `Found ${ports.length} serial ports.` : 'No serial ports found.',
      },
    }))
  },
  connect: async () => {
    const { config } = get()
    if (!config.port) {
      set({ status: { tone: 'warning', message: 'Choose a serial port first.' } })
      return
    }

    const session = await connectSerial(config)
    set({
      session,
      status: {
        tone: 'neutral',
        message: `Opening ${config.port} at ${config.baudRate} baud...`,
      },
    })
  },
  disconnect: async () => {
    const session = await disconnectSerial()
    set({
      session,
      status: { tone: 'neutral', message: 'Serial session stopped.' },
    })
  },
  send: async () => {
    const { commandInput, appendNewline } = get()
    const payload = commandInput.trim()
    if (!payload) {
      set({ status: { tone: 'warning', message: 'Enter a payload before sending.' } })
      return
    }

    await sendSerial({ payload, appendNewline })
    set({
      commandInput: '',
      status: { tone: 'success', message: `Sent ${payload}.` },
    })
  },
  ingestEvent: (event) => {
    if (event.kind === 'connection') {
      set({
        session: {
          isRunning: event.connection.state !== 'stopped',
          port: event.source.port,
          baudRate: event.source.baudRate,
          connectionState: event.connection.state,
        },
        status: {
          tone: connectionTone(event.connection.state),
          message: connectionMessage(event),
        },
      })
      return
    }

    set((state) => {
      const consoleEntries = [...state.consoleEntries, asConsoleEntry(event)].slice(-MAX_CONSOLE_ENTRIES)
      const nextState: Partial<AppStore> = { consoleEntries }
      const channelId = event.parser.fields.channelId
      const rawValue = event.parser.fields.value

      if (event.line.direction === 'rx' && channelId && rawValue) {
        const numericValue = Number(rawValue)
        const previous = state.variables[channelId]
        const trend = computeTrend(previous?.numericValue, numericValue)
        const points = Number.isFinite(numericValue)
          ? [...(previous?.points ?? []), { timestampMs: event.timestampMs, value: numericValue }].slice(
              -MAX_SAMPLES_PER_CHANNEL,
            )
          : previous?.points ?? []
        const variables = {
          ...state.variables,
          [channelId]: {
            name: channelId,
            currentValue: rawValue,
            previousValue: previous?.numericValue,
            numericValue: Number.isFinite(numericValue) ? numericValue : previous?.numericValue,
            trend,
            parserName: event.parser.parserName,
            sampleCount: (previous?.sampleCount ?? 0) + 1,
            updatedAtMs: event.timestampMs,
            points,
          },
        }
        nextState.variables = variables
        nextState.selectedChannels = autoSelectChannels(state.selectedChannels, channelId)
      }

      return nextState as AppStore
    })
  },
  toggleChannel: (channel) => {
    set((state) => ({
      selectedChannels: state.selectedChannels.includes(channel)
        ? state.selectedChannels.filter((name) => name !== channel)
        : [...state.selectedChannels, channel].slice(-4),
    }))
  },
  colorForChannel: (channel) => colorForChannel(channel),
}))

const asConsoleEntry = (event: Extract<SerialBusEvent, { kind: 'line' }>): ConsoleEntry => ({
  id: `${event.timestampMs}-${event.line.direction}-${event.line.text}`,
  direction: event.line.direction,
  text: event.line.text,
  timestampMs: event.timestampMs,
})

const autoSelectChannels = (selectedChannels: string[], channelId: string) => {
  if (selectedChannels.includes(channelId)) {
    return selectedChannels
  }
  if (selectedChannels.length >= 3) {
    return selectedChannels
  }
  return [...selectedChannels, channelId]
}

const computeTrend = (previousValue: number | undefined, numericValue: number) => {
  if (!Number.isFinite(numericValue) || previousValue === undefined) {
    return 'flat' as const
  }
  if (numericValue > previousValue) {
    return 'up' as const
  }
  if (numericValue < previousValue) {
    return 'down' as const
  }
  return 'flat' as const
}

const connectionTone = (state: SessionSnapshot['connectionState']): AppStatus['tone'] => {
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

const connectionMessage = (event: Extract<SerialBusEvent, { kind: 'connection' }>) => {
  const label = `${event.source.port} @ ${event.source.baudRate}`

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

const colorForChannel = (channel: string) => {
  let hash = 0
  for (const char of channel) {
    hash = (hash << 5) - hash + char.charCodeAt(0)
    hash |= 0
  }
  return CHANNEL_COLORS[Math.abs(hash) % CHANNEL_COLORS.length]
}
