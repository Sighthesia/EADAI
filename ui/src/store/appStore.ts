import { create } from 'zustand'
import {
  connectSerial,
  disconnectSerial,
  getMcpServerStatus,
  getSessionSnapshot,
  listSerialPorts,
  sendSerial,
} from '../lib/tauri'
import type {
  AppStatus,
  ConnectRequest,
  ConsoleEntry,
  McpServerStatus,
  SerialBusEvent,
  SessionSnapshot,
  UiAnalysisPayload,
  UiTriggerPayload,
  VariableEntry,
} from '../types'

const MAX_CONSOLE_ENTRIES = 400
const MAX_SAMPLES_PER_CHANNEL = 960
const CHANNEL_COLORS = ['#4FC3F7', '#C792EA', '#F78C6C', '#A5E075', '#E6C07B', '#82AAFF']
const DEFAULT_FAKE_PROFILE = 'telemetry-lab'

type AppStore = {
  ports: string[]
  session: SessionSnapshot
  mcp: McpServerStatus
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
  refreshMcpStatus: () => Promise<void>
  refreshPorts: () => Promise<void>
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  send: () => Promise<void>
  ingestEvent: (event: SerialBusEvent) => void
  ingestEvents: (events: SerialBusEvent[]) => void
  toggleChannel: (channel: string) => void
  colorForChannel: (channel: string) => string
}

const defaultStatus = (): AppStatus => ({
  tone: 'neutral',
  message: 'Ready. Fake stream will autostart for UI debugging.',
})

const MCP_STATUS_POLL_INTERVAL_MS = 1000

export const useAppStore = create<AppStore>((set, get) => ({
  ports: [],
  session: { isRunning: false, connectionState: 'stopped' },
  mcp: {
    isRunning: false,
    transport: 'streamableHttp',
    endpointUrl: null,
    lastError: null,
  },
  config: {
    port: '',
    baudRate: 115200,
    retryMs: 1000,
    readTimeoutMs: 50,
    sourceKind: 'fake',
    fakeProfile: DEFAULT_FAKE_PROFILE,
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
    const [ports, session, mcp] = await Promise.all([
      listSerialPorts(),
      getSessionSnapshot(),
      readMcpStatusWithRetry(),
    ])
    set((state) => ({
      ports,
      session,
      mcp,
      config: {
        ...state.config,
        port:
          session.transport === 'fake'
            ? state.config.port
            : session.port ?? ports[0] ?? state.config.port,
        baudRate: session.baudRate ?? state.config.baudRate,
        sourceKind:
          session.transport === 'fake'
            ? 'fake'
            : session.transport === 'serial'
              ? 'serial'
              : state.config.sourceKind,
      },
      status: session.isRunning
        ? {
            tone: 'success',
            message: `Session on ${session.port ?? 'serial'} is ${session.connectionState ?? 'running'}.`,
          }
        : defaultStatus(),
    }))
  },
  refreshMcpStatus: async () => {
    const mcp = await readMcpStatusWithRetry()
    set({ mcp })
  },
  refreshPorts: async () => {
    const ports = await listSerialPorts()
    set((state) => ({
      ports,
      config: {
        ...state.config,
        port:
          state.config.sourceKind === 'fake'
            ? state.config.port
            : ports.includes(state.config.port)
              ? state.config.port
              : ports[0] ?? '',
      },
      status: {
        tone: 'neutral',
        message: ports.length > 0 ? `Found ${ports.length} serial ports.` : 'No serial ports found.',
      },
    }))
  },
  connect: async () => {
    const { config } = get()
    if (config.sourceKind === 'serial' && !config.port) {
      set({ status: { tone: 'warning', message: 'Choose a serial port first.' } })
      return
    }

    const request =
      config.sourceKind === 'fake'
        ? {
            ...config,
            port: fakePortLabel(config.fakeProfile ?? DEFAULT_FAKE_PROFILE),
            fakeProfile: config.fakeProfile ?? DEFAULT_FAKE_PROFILE,
          }
        : config

    const session = await connectSerial(request)
    set({
      session,
      status: {
        tone: 'neutral',
        message:
          config.sourceKind === 'fake'
            ? `Starting ${request.fakeProfile} fake stream...`
            : `Opening ${config.port} at ${config.baudRate} baud...`,
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
  ingestEvent: (event) => get().ingestEvents([event]),
  ingestEvents: (events) => {
    if (events.length === 0) {
      return
    }

    set((state) => {
      let consoleEntries = state.consoleEntries
      let variables = state.variables
      let selectedChannels = state.selectedChannels
      let session = state.session
      let status = state.status
      let variablesChanged = false
      let selectedChannelsChanged = false
      let consoleChanged = false
      let sessionChanged = false
      let statusChanged = false

      for (const event of events) {
        if (event.kind === 'connection') {
          session = {
            isRunning: event.connection.state !== 'stopped',
            transport: event.source.transport,
            port: event.source.port,
            baudRate: event.source.baudRate,
            connectionState: event.connection.state,
          }
          status = {
            tone: connectionTone(event.connection.state),
            message: connectionMessage(event),
          }
          sessionChanged = true
          statusChanged = true
          continue
        }

        if (event.kind === 'line') {
          consoleEntries = [...consoleEntries, asConsoleEntry(event)].slice(-MAX_CONSOLE_ENTRIES)
          consoleChanged = true

          const channelId = event.parser.fields.channelId
          const rawValue = event.parser.fields.value
          if (event.line.direction !== 'rx' || !channelId || !rawValue) {
            continue
          }

          const sampleTimestampMs = parseSampleTimestampMs(event.parser.fields.timestamp, event.timestampMs)
          const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs)
          const numericRaw = event.parser.fields.numericValue ?? rawValue
          const numericValue = Number(numericRaw)
          const nextPoints = Number.isFinite(numericValue)
            ? [...previous.points, { timestampMs: sampleTimestampMs, value: numericValue }].slice(-MAX_SAMPLES_PER_CHANNEL)
            : previous.points

          if (!variablesChanged) {
            variables = { ...variables }
            variablesChanged = true
          }

          variables[channelId] = {
            ...previous,
            name: channelId,
            currentValue: rawValue,
            previousValue: previous.numericValue,
            numericValue: Number.isFinite(numericValue) ? numericValue : previous.numericValue,
            parserName: event.parser.parserName,
            sampleCount: previous.sampleCount + 1,
            updatedAtMs: event.timestampMs,
            points: nextPoints,
          }

          const nextSelectedChannels = autoSelectChannels(selectedChannels, channelId)
          if (nextSelectedChannels !== selectedChannels) {
            selectedChannels = nextSelectedChannels
            selectedChannelsChanged = true
          }
          continue
        }

        if (event.kind === 'analysis') {
          const channelId = event.analysis.channelId
          const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs)

          if (!variablesChanged) {
            variables = { ...variables }
            variablesChanged = true
          }

          variables[channelId] = applyAnalysis(previous, event.analysis, event.timestampMs)

          const nextSelectedChannels = autoSelectChannels(selectedChannels, channelId)
          if (nextSelectedChannels !== selectedChannels) {
            selectedChannels = nextSelectedChannels
            selectedChannelsChanged = true
          }
          continue
        }

        const channelId = event.trigger.channelId
        const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs)

        if (!variablesChanged) {
          variables = { ...variables }
          variablesChanged = true
        }

        variables[channelId] = {
          ...previous,
          latestTrigger: event.trigger,
          triggerCount: previous.triggerCount + 1,
          updatedAtMs: event.timestampMs,
        }

        const nextSelectedChannels = autoSelectChannels(selectedChannels, channelId)
        if (nextSelectedChannels !== selectedChannels) {
          selectedChannels = nextSelectedChannels
          selectedChannelsChanged = true
        }

        status = {
          tone: severityTone(event.trigger.severity),
          message: triggerMessage(event.trigger),
        }
        statusChanged = true
      }

      const nextState: Partial<AppStore> = {}
      if (consoleChanged) {
        nextState.consoleEntries = consoleEntries
      }
      if (variablesChanged) {
        nextState.variables = variables
      }
      if (selectedChannelsChanged) {
        nextState.selectedChannels = selectedChannels
      }
      if (sessionChanged) {
        nextState.session = session
      }
      if (statusChanged) {
        nextState.status = status
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

const createVariableEntry = (channelId: string, updatedAtMs: number): VariableEntry => ({
  name: channelId,
  currentValue: '—',
  trend: 'flat',
  parserName: null,
  sampleCount: 0,
  updatedAtMs,
  points: [],
  analysis: null,
  latestTrigger: null,
  triggerCount: 0,
})

const applyAnalysis = (
  variable: VariableEntry,
  analysis: UiAnalysisPayload,
  updatedAtMs: number,
): VariableEntry => ({
  ...variable,
  analysis,
  trend: trendFromDelta(analysis.trend),
  updatedAtMs,
})

const autoSelectChannels = (selectedChannels: string[], channelId: string) => {
  if (selectedChannels.includes(channelId)) {
    return selectedChannels
  }
  if (selectedChannels.length >= 4) {
    return selectedChannels
  }
  return [...selectedChannels, channelId]
}

const trendFromDelta = (delta: number | null | undefined) => {
  if (delta === undefined || delta === null) {
    return 'flat' as const
  }
  if (delta > 0) {
    return 'up' as const
  }
  if (delta < 0) {
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

const readMcpStatusWithRetry = async () => {
  let lastStatus = await getMcpServerStatus()

  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (lastStatus.isRunning || lastStatus.lastError) {
      return lastStatus
    }

    await new Promise((resolve) => window.setTimeout(resolve, 120))
    lastStatus = await getMcpServerStatus()
  }

  return lastStatus
}

export const shouldPollMcpStatus = (mcp: McpServerStatus) => !mcp.isRunning && !mcp.lastError

const connectionMessage = (event: Extract<SerialBusEvent, { kind: 'connection' }>) => {
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

const severityTone = (severity: UiTriggerPayload['severity']): AppStatus['tone'] => {
  switch (severity) {
    case 'critical':
      return 'danger'
    case 'warning':
      return 'warning'
    case 'info':
      return 'neutral'
  }
}

const triggerMessage = (trigger: UiTriggerPayload) =>
  `Trigger ${trigger.ruleId} on ${trigger.channelId}: ${trigger.reason}`

const fakePortLabel = (profile: string) => `fake://${profile}`

const parseSampleTimestampMs = (timestamp: string | undefined, fallback: number) => {
  if (!timestamp) {
    return fallback
  }

  const parsed = Number(timestamp)
  return Number.isFinite(parsed) ? parsed : fallback
}

const colorForChannel = (channel: string) => {
  let hash = 0
  for (const char of channel) {
    hash = (hash << 5) - hash + char.charCodeAt(0)
    hash |= 0
  }
  return CHANNEL_COLORS[Math.abs(hash) % CHANNEL_COLORS.length]
}
