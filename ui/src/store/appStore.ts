import { create } from 'zustand'
import {
  autoDetectImuAttitudeMap,
  autoDetectImuChannelMap,
  autoDetectImuQuaternionMap,
  createEmptyImuAttitudeMap,
  createEmptyImuChannelMap,
  createEmptyImuQuaternionMap,
} from '../lib/imu'
import {
  readWaveformVisualAidState,
  setWaveformVisualAidPreference,
  writeWaveformVisualAidState,
  type WaveformVisualAidKey,
  type WaveformVisualAidState,
} from '../lib/waveformVisualAids'
import {
  connectSerial,
  disconnectSerial,
  getMcpToolUsageSnapshot,
  getSessionSnapshot,
  listSerialPorts,
  sendBmi088Command,
  sendSerial,
  startLogicAnalyzerCapture,
  stopLogicAnalyzerCapture,
} from '../lib/tauri'
import { clampWaveformWindowMs, DEFAULT_WAVEFORM_WINDOW_MS } from '../lib/waveformWindow'
import { createDevTimingLogger, logger } from '../lib/logger'
import type {
  AppStatus,
  Bmi088HostCommand,
  ConsoleDisplayMode,
  ConnectRequest,
  ConsoleEntry,
  ImuAttitudeMap,
  ImuAttitudeRole,
  ImuChannelMap,
  ImuChannelRole,
  ImuMapMode,
  ImuOrientationSource,
  ImuQuaternionMap,
  ImuQuaternionRole,
  ImuCalibrationState,
  ImuQualitySnapshot,
  LogicAnalyzerConfig,
  LogicAnalyzerStatus,
  McpServerStatus,
  McpToolUsageSnapshot,
  SerialBusEvent,
  SerialDeviceInfo,
  SessionSnapshot,
  UiProtocolSnapshot,
  UiRuntimeCatalogSnapshot,
  UiRuntimeDeviceSnapshot,
  UiScriptsDefinitionModel,
  UiScriptHookExample,
  VariableEntry,
  VariableDefinition,
} from '../types'

// -- Module imports --
import {
  DEFAULT_FAKE_PROFILE,
  DEFAULT_LOGIC_SAMPLE_COUNT,
  MAX_CONSOLE_ENTRIES,
  MAX_SENT_CONSOLE_ENTRIES,
  MAX_SAMPLES_PER_CHANNEL,
} from './constants'
import {
  asConsoleEntry,
  asProtocolConsoleEntry,
  appendConsoleHistory,
  appendConsoleHistoryBatch,
  createSentConsoleEntry,
} from './consoleHelpers'
import {
  applyAnalysis,
  autoSelectChannels,
  colorForChannel,
  createVariableEntry,
  fakePortLabel,
  parseSampleTimestampMs,
  severityTone,
  triggerMessage,
  updateVariableFromProtocolText,
  updateVariableFromTelemetrySample,
} from './eventIngestHelpers'
import {
  defaultLogicAnalyzerStatus,
  parseLogicSamplerate,
  readLogicAnalyzerStatusSafely,
  refreshLogicAnalyzerDevicesSafely,
  syncLogicAnalyzerConfig,
} from './logicAnalyzerHelpers'
import {
  computeImuQuality,
  isSameImuAttitudeMap,
  isSameImuChannelMap,
  isSameImuQualitySnapshot,
  isSameImuQuaternionMap,
} from './imuHelpers'
import {
  connectionMessage,
  connectionTone,
  defaultProtocolSnapshot,
  ingestProtocolConnection,
  ingestProtocolIdentity,
  ingestProtocolLine,
  ingestProtocolSample,
  ingestProtocolSchema,
  ingestProtocolShellOutput,
} from './protocolHelpers'
import {
  createRuntimeCatalog,
  createRuntimeDeviceSnapshot,
  runtimeDeviceRef,
} from './runtimeHelpers'
import {
  buildPortsRefreshState,
  readMcpStatusWithRetry,
  shouldPollMcpStatus,
} from './sessionHelpers'
import {
  buildCrtpConsoleText,
  buildCrtpDisplayValue,
  buildMavlinkConsoleText,
  buildMavlinkDisplayValue,
} from './protocolDisplayHelpers'
import {
  createProtocolHookExamples,
  createScriptDefinitions,
  syncScriptDefinitions,
} from './variableHelpers'

// Re-export shouldPollMcpStatus so existing callers keep working.
export { shouldPollMcpStatus } from './sessionHelpers'

const profileIngestEvents = createDevTimingLogger('appStore.ingestEvents', { slowThresholdMs: 8, summaryEvery: 120, summaryIntervalMs: 5_000 })

type AppStore = {
  ports: SerialDeviceInfo[]
  session: SessionSnapshot
  mcp: McpServerStatus
  mcpToolUsage: McpToolUsageSnapshot[]
  config: ConnectRequest
  appendNewline: boolean
  commandInput: string
  consoleDisplayMode: ConsoleDisplayMode
  consoleEntries: ConsoleEntry[]
  sentConsoleEntries: ConsoleEntry[]
  protocol: UiProtocolSnapshot
  runtimeDevice: UiRuntimeDeviceSnapshot
  variables: Record<string, VariableEntry>
  selectedChannels: string[]
  waveformWindowMs: number
  visualAidState: WaveformVisualAidState
  imuChannelMap: ImuChannelMap
  imuAttitudeMap: ImuAttitudeMap
  imuQuaternionMap: ImuQuaternionMap
  imuMapMode: ImuMapMode
  imuOrientationSource: ImuOrientationSource
  imuCalibration: ImuCalibrationState
  imuQuality: ImuQualitySnapshot
  logicAnalyzer: LogicAnalyzerStatus
  logicAnalyzerConfig: LogicAnalyzerConfig
  runtimeCatalog: UiRuntimeCatalogSnapshot
  scriptDefinitions: UiScriptsDefinitionModel
  status: AppStatus
  updateVariableDefinition: (definitionId: string, patch: Partial<Pick<VariableDefinition, 'bindingField' | 'alias' | 'presentationUnit' | 'visibility' | 'sourceLabel' | 'summary'>>) => void
  setCommandInput: (value: string) => void
  setAppendNewline: (value: boolean) => void
  setConsoleDisplayMode: (value: ConsoleDisplayMode) => void
  setWaveformWindowMs: (value: number | ((current: number) => number)) => void
  patchConfig: (value: Partial<ConnectRequest>) => void
  bootstrap: () => Promise<void>
  refreshMcpStatus: () => Promise<void>
  refreshMcpToolUsage: () => Promise<void>
  refreshPorts: () => Promise<void>
  refreshPortsSilently: () => Promise<void>
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  send: () => Promise<void>
  sendBmi088Command: (command: Bmi088HostCommand, payload?: string | null) => Promise<void>
  ingestEvent: (event: SerialBusEvent) => void
  ingestEvents: (events: SerialBusEvent[]) => void
  toggleChannel: (channel: string) => void
  setVisualAidEnabled: (channel: string, key: WaveformVisualAidKey, enabled: boolean) => void
  setImuChannel: (role: ImuChannelRole, channel: string | null) => void
  setImuAttitudeChannel: (role: ImuAttitudeRole, channel: string | null) => void
  setImuQuaternionChannel: (role: ImuQuaternionRole, channel: string | null) => void
  autoMapImuChannels: () => void
  setImuOrientationSource: (value: ImuOrientationSource) => void
  calibrateImuFromCurrentState: () => void
  resetImuCalibration: () => void
  refreshLogicAnalyzerStatus: () => Promise<void>
  refreshLogicAnalyzerDevices: () => Promise<void>
  patchLogicAnalyzerConfig: (value: Partial<LogicAnalyzerConfig>) => void
  toggleLogicAnalyzerChannel: (channel: string) => void
  startLogicAnalyzerCapture: () => Promise<void>
  stopLogicAnalyzerCapture: () => Promise<void>
  colorForChannel: (channel: string) => string
  protocolHookExamples: UiScriptHookExample[]
}

const defaultStatus = (): AppStatus => ({
  tone: 'neutral',
  message: 'Ready. Fake stream will autostart for UI debugging.',
})

export const useAppStore = create<AppStore>((set, get) => ({
  ports: [],
  session: { isRunning: false, connectionState: 'stopped' },
  mcp: {
    isRunning: false,
    transport: 'streamableHttp',
    endpointUrl: null,
    lastError: null,
  },
  mcpToolUsage: [],
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
  consoleDisplayMode: 'ascii' as const,
  consoleEntries: [],
  sentConsoleEntries: [],
  protocol: defaultProtocolSnapshot(),
  protocolHookExamples: createProtocolHookExamples(),
  variables: {},
  selectedChannels: [],
  waveformWindowMs: DEFAULT_WAVEFORM_WINDOW_MS,
  visualAidState: readWaveformVisualAidState(),
  imuChannelMap: createEmptyImuChannelMap(),
  imuAttitudeMap: createEmptyImuAttitudeMap(),
  imuQuaternionMap: createEmptyImuQuaternionMap(),
  imuMapMode: 'auto',
  imuOrientationSource: 'rawFusion',
  imuCalibration: {
    accelBiasApplied: false,
    gyroBiasApplied: false,
    sourceLabel: null,
    lastCalibratedAtMs: null,
  },
  imuQuality: {
    level: 'idle',
    label: 'No IMU data',
    details: 'Waiting for mapped IMU samples.',
    timestampMs: null,
  },
  logicAnalyzer: defaultLogicAnalyzerStatus(),
  logicAnalyzerConfig: {
    deviceRef: '',
    sampleCount: DEFAULT_LOGIC_SAMPLE_COUNT,
    samplerateHzInput: '',
    selectedChannelLabels: [],
  },
  runtimeDevice: createRuntimeDeviceSnapshot(
    { isRunning: false, connectionState: 'stopped' },
    {
      port: '',
      baudRate: 115200,
      retryMs: 1000,
      readTimeoutMs: 50,
      sourceKind: 'fake',
      fakeProfile: DEFAULT_FAKE_PROFILE,
    },
    [],
  ),
  runtimeCatalog: createRuntimeCatalog(
    defaultProtocolSnapshot(),
    createRuntimeDeviceSnapshot(
      { isRunning: false, connectionState: 'stopped' },
      {
        port: '',
        baudRate: 115200,
        retryMs: 1000,
        readTimeoutMs: 50,
        sourceKind: 'fake',
        fakeProfile: DEFAULT_FAKE_PROFILE,
      },
      [],
    ),
  ),
  scriptDefinitions: createScriptDefinitions({}),
  status: defaultStatus(),
  updateVariableDefinition: (definitionId, patch) =>
    set((state) => ({
      scriptDefinitions: {
        ...state.scriptDefinitions,
        variables: state.scriptDefinitions.variables.map((definition) =>
          definition.id === definitionId
            ? {
                ...definition,
                ...patch,
                updatedAtMs: Date.now(),
                status: 'draft',
              }
            : definition,
        ),
      },
    })),
  setCommandInput: (value) => set({ commandInput: value }),
  setAppendNewline: (value) => set({ appendNewline: value }),
  setConsoleDisplayMode: (value) => set({ consoleDisplayMode: value }),
  setWaveformWindowMs: (value) =>
    set((state) => ({
      waveformWindowMs: clampWaveformWindowMs(typeof value === 'function' ? value(state.waveformWindowMs) : value),
    })),
  patchConfig: (value) =>
    set((state) => {
      const config = { ...state.config, ...value }
      const runtimeDevice = createRuntimeDeviceSnapshot(state.session, config, state.ports)
      return {
        config,
        runtimeDevice,
        runtimeCatalog: createRuntimeCatalog(state.protocol, runtimeDevice),
      }
    }),
  bootstrap: async () => {
    const [ports, session, mcp, mcpToolUsage, logicAnalyzer] = await Promise.all([
      listSerialPorts(),
      getSessionSnapshot(),
      readMcpStatusWithRetry(),
      getMcpToolUsageSnapshot(),
      refreshLogicAnalyzerDevicesSafely(),
    ])
    logger.debug('bootstrap', { portsLength: ports.length })
    set((state) => ({
      ports,
      session,
      mcp,
      mcpToolUsage,
      logicAnalyzer,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, logicAnalyzer),
      runtimeDevice: createRuntimeDeviceSnapshot(session, state.config, ports),
      runtimeCatalog: createRuntimeCatalog(state.protocol, createRuntimeDeviceSnapshot(session, state.config, ports)),
      config: {
        ...state.config,
        port:
          session.transport === 'fake'
            ? state.config.port
            : session.port ?? ports[0]?.portName ?? state.config.port,
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
  refreshMcpToolUsage: async () => {
    const mcpToolUsage = await getMcpToolUsageSnapshot()
    set({ mcpToolUsage })
  },
  refreshLogicAnalyzerStatus: async () => {
    const logicAnalyzer = await readLogicAnalyzerStatusSafely()
    set((state) => ({
      logicAnalyzer,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, logicAnalyzer),
    }))
  },
  refreshLogicAnalyzerDevices: async () => {
    const logicAnalyzer = await refreshLogicAnalyzerDevicesSafely()
    set((state) => ({
      logicAnalyzer,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, logicAnalyzer),
    }))
  },
  patchLogicAnalyzerConfig: (value) =>
    set((state) => ({
      logicAnalyzerConfig: {
        ...state.logicAnalyzerConfig,
        ...value,
      },
    })),
  toggleLogicAnalyzerChannel: (channel) => {
    set((state) => ({
      logicAnalyzerConfig: {
        ...state.logicAnalyzerConfig,
        selectedChannelLabels: state.logicAnalyzerConfig.selectedChannelLabels.includes(channel)
          ? state.logicAnalyzerConfig.selectedChannelLabels.filter((name) => name !== channel)
          : [...state.logicAnalyzerConfig.selectedChannelLabels, channel],
      },
    }))
  },
  startLogicAnalyzerCapture: async () => {
    const { logicAnalyzer, logicAnalyzerConfig } = get()
    const deviceRef = logicAnalyzerConfig.deviceRef.trim()

    if (!deviceRef) {
      set((state) => ({
        logicAnalyzer: {
          ...state.logicAnalyzer,
          lastError: 'Select a logic analyzer device first.',
        },
      }))
      return
    }

    const nextStatus = await startLogicAnalyzerCapture({
      deviceRef,
      sampleCount: Math.max(1, logicAnalyzerConfig.sampleCount || DEFAULT_LOGIC_SAMPLE_COUNT),
      samplerateHz: parseLogicSamplerate(logicAnalyzerConfig.samplerateHzInput),
      channels: logicAnalyzerConfig.selectedChannelLabels,
    })

    set((state) => ({
      logicAnalyzer: nextStatus,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, nextStatus),
      status: {
        tone: 'neutral',
        message: `Starting logic capture on ${logicAnalyzer.devices.find((device) => device.reference === deviceRef)?.name ?? deviceRef}.`,
      },
    }))
  },
  stopLogicAnalyzerCapture: async () => {
    const nextStatus = await stopLogicAnalyzerCapture()
    set((state) => ({
      logicAnalyzer: nextStatus,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, nextStatus),
      status: {
        tone: nextStatus.lastCapture ? 'success' : 'neutral',
        message: nextStatus.lastCapture ? 'Logic capture completed.' : 'Logic capture stopped.',
      },
    }))
  },
  refreshPorts: async () => {
    const ports = await listSerialPorts()
    logger.debug('refreshPorts', { portsLength: ports.length })
    set((state) => buildPortsRefreshState(state, ports, false))
  },
  refreshPortsSilently: async () => {
    const ports = await listSerialPorts()
    logger.debug('refreshPortsSilently', { portsLength: ports.length })
    set((state) => buildPortsRefreshState(state, ports, true))
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
      runtimeDevice: createRuntimeDeviceSnapshot(session, config, []),
      variables: {},
      selectedChannels: [],
      consoleEntries: [],
      sentConsoleEntries: [],
      protocol: defaultProtocolSnapshot(),
      runtimeCatalog: createRuntimeCatalog(defaultProtocolSnapshot(), createRuntimeDeviceSnapshot(session, config, [])),
      scriptDefinitions: createScriptDefinitions({}),
      imuQuality: {
        level: 'idle',
        label: 'No IMU data',
        details: 'Waiting for mapped IMU samples.',
        timestampMs: null,
      },
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
      runtimeDevice: createRuntimeDeviceSnapshot(session, get().config, get().ports),
      status: { tone: 'neutral', message: 'Serial session stopped.' },
      variables: {},
      selectedChannels: [],
      consoleEntries: [],
      sentConsoleEntries: [],
      protocol: defaultProtocolSnapshot(),
      runtimeCatalog: createRuntimeCatalog(defaultProtocolSnapshot(), createRuntimeDeviceSnapshot(session, get().config, get().ports)),
      scriptDefinitions: createScriptDefinitions({}),
      imuQuality: {
        level: 'idle',
        label: 'No IMU data',
        details: 'Waiting for mapped IMU samples.',
        timestampMs: null,
      },
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
      sentConsoleEntries: appendConsoleHistory(get().sentConsoleEntries, createSentConsoleEntry(payload, appendNewline), MAX_SENT_CONSOLE_ENTRIES),
      status: { tone: 'success', message: `Sent ${payload}.` },
    })
  },
  sendBmi088Command: async (command, payload) => {
    await sendBmi088Command({ command, payload: payload ?? null })
    const text = payload?.trim().length ? `${command} ${payload.trim()}` : command
    set({
      sentConsoleEntries: appendConsoleHistory(get().sentConsoleEntries, createSentConsoleEntry(text, false), MAX_SENT_CONSOLE_ENTRIES),
      status: { tone: 'success', message: `Sent BMI088 ${command}.` },
    })
  },
  ingestEvent: (event) => get().ingestEvents([event]),
  ingestEvents: (events) => {
    if (events.length === 0) {
      return
    }

    const startedAtMs = performance.now()

    try {
      set((state) => {
        let consoleEntries = state.consoleEntries
        const pendingConsoleEntries: ConsoleEntry[] = []
        let variables = state.variables
        let selectedChannels = state.selectedChannels
        let imuChannelMap = state.imuChannelMap
        let imuAttitudeMap = state.imuAttitudeMap
        let imuQuaternionMap = state.imuQuaternionMap
        let imuCalibration = state.imuCalibration
        let imuQuality = state.imuQuality
        let session = state.session
        let status = state.status
        let protocol = state.protocol
        let scriptDefinitions = state.scriptDefinitions
        let variablesChanged = false
        let selectedChannelsChanged = false
        let imuChannelMapChanged = false
        let imuAttitudeMapChanged = false
        let imuQuaternionMapChanged = false
        let imuQualityChanged = false
        let consoleChanged = false
        let sessionChanged = false
        let statusChanged = false
        let protocolChanged = false
        let shouldRecomputeImuQuality = false

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
            protocol = ingestProtocolConnection(protocol, event)
            sessionChanged = true
            statusChanged = true
            protocolChanged = true
            continue
          }

          if (event.kind === 'line') {
            const consoleEntry = asConsoleEntry(event)
            pendingConsoleEntries.push(consoleEntry)

            protocol = ingestProtocolLine(protocol, event)
            protocolChanged = true

            const channelId = event.parser.fields.channelId
            const rawValue = event.parser.fields.value
            if (event.line.direction !== 'rx' || !channelId || !rawValue) {
              continue
            }

            const sampleTimestampMs = parseSampleTimestampMs(event.parser.fields.timestamp, event.timestampMs)
            const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs, runtimeDeviceRef(session, state.config), 'protocol-text')
            const numericRaw = event.parser.fields.numericValue ?? rawValue
            const numericValue = Number(numericRaw)
            const nextPoints = Number.isFinite(numericValue)
              ? [...previous.points, { timestampMs: sampleTimestampMs, value: numericValue }].slice(-MAX_SAMPLES_PER_CHANNEL)
              : previous.points

            if (!variablesChanged) {
              variables = { ...variables }
              variablesChanged = true
            }
            variables[channelId] = updateVariableFromProtocolText(previous, channelId, event, rawValue, numericValue, nextPoints)

            const nextSelectedChannels = autoSelectChannels(selectedChannels, channelId)
            if (nextSelectedChannels !== selectedChannels) {
              selectedChannels = nextSelectedChannels
              selectedChannelsChanged = true
            }

            shouldRecomputeImuQuality = true
            continue
          }

          if (event.kind === 'shellOutput') {
            pendingConsoleEntries.push(asConsoleEntry({
              ...event,
              kind: 'line',
            } as Extract<SerialBusEvent, { kind: 'line' }>))
            protocol = ingestProtocolShellOutput(protocol, event)
            protocolChanged = true
            continue
          }

          if (event.kind === 'analysis') {
            const channelId = event.analysis.channelId
            const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs, runtimeDeviceRef(session, state.config))

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

          if (event.kind === 'telemetrySchema') {
            pendingConsoleEntries.push(asProtocolConsoleEntry(event, `SCHEMA ${event.schema.fields.length} fields @ ${event.schema.rateHz}Hz`))
            protocol = ingestProtocolSchema(protocol, event)
            protocolChanged = true

            const nextVariables = { ...state.variables }
            let changed = false

            event.schema.fields.forEach((field, index) => {
              if (!nextVariables[field.name]) {
                nextVariables[field.name] = createVariableEntry(field.name, event.timestampMs, runtimeDeviceRef(session, state.config), 'telemetry-sample')
              }

              nextVariables[field.name] = {
                ...nextVariables[field.name],
                name: field.name,
                unit: field.unit,
                sourceKind: 'telemetry-sample',
                parserName: 'bmi088_schema',
                updatedAtMs: event.timestampMs,
                sampleCount: nextVariables[field.name].sampleCount,
              }
              changed = true

              if (index === 0 && selectedChannels.length === 0) {
                selectedChannels = [field.name]
                selectedChannelsChanged = true
              }
            })

            if (changed) {
              variables = nextVariables
              variablesChanged = true
            }
            continue
          }

          if (event.kind === 'telemetryIdentity') {
            pendingConsoleEntries.push(
              asProtocolConsoleEntry(
                event,
                `IDENTITY ${event.identity.deviceName} ${event.identity.boardName} ${event.identity.firmwareVersion}`,
              ),
            )
            protocol = ingestProtocolIdentity(protocol, event)
            protocolChanged = true
            continue
          }

          if (event.kind === 'telemetrySample') {
            pendingConsoleEntries.push(asProtocolConsoleEntry(event, `SAMPLE ${event.sample.fields.length} fields`))
            protocol = ingestProtocolSample(protocol, event)
            protocolChanged = true

            const schemaFields = event.sample.fields
            if (!variablesChanged) {
              variables = { ...variables }
              variablesChanged = true
            }

            for (const field of schemaFields) {
              const previous = variables[field.name] ?? createVariableEntry(field.name, event.timestampMs, runtimeDeviceRef(session, state.config), 'telemetry-sample')
              const nextNumericValue = field.value
              const displayValue = field.value.toFixed(field.scaleQ < 0 ? 2 : 6)

              variables[field.name] = updateVariableFromTelemetrySample(previous, field.name, event, nextNumericValue, displayValue, field.unit ?? previous.unit ?? null)
            }

            const nextSelectedChannels = autoSelectChannels(selectedChannels, schemaFields[0]?.name ?? 'acc_x')
            if (nextSelectedChannels !== selectedChannels) {
              selectedChannels = nextSelectedChannels
              selectedChannelsChanged = true
            }

            shouldRecomputeImuQuality = true
            continue
          }

          if (event.kind === 'mavlinkPacket') {
            const messageIdHex = `0x${event.packet.messageId.toString(16).padStart(4, '0')}`
            const channelKey = `mavlink.msg_${messageIdHex}`
            
            // Build rich display value from semantic fields
            const displayValue = buildMavlinkDisplayValue(event.packet)
            const previous = variables[channelKey] ?? createVariableEntry(channelKey, event.timestampMs, runtimeDeviceRef(session, state.config), 'protocol-text')

            if (!variablesChanged) {
              variables = { ...variables }
              variablesChanged = true
            }

            variables[channelKey] = {
              ...previous,
              name: channelKey,
              sourceKind: 'protocol-text',
              currentValue: displayValue,
              parserName: 'mavlink',
              sampleCount: previous.sampleCount + 1,
              updatedAtMs: event.timestampMs,
            }

            // Build rich console text from semantic fields
            const consoleText = buildMavlinkConsoleText(event.packet)
            pendingConsoleEntries.push(asConsoleEntry({
              kind: 'line',
              timestampMs: event.timestampMs,
              source: event.source,
              line: { direction: 'rx', text: consoleText, raw: [], rawLength: event.packet.payloadLen },
              parser: { parserName: 'mavlink', status: 'parsed', fields: event.packet.fields },
            } as Extract<SerialBusEvent, { kind: 'line' }>))

            continue
          }

          if (event.kind === 'crtpPacket') {
            const channelKey = `crtp.${event.packet.port}.ch${event.packet.channel}`
            
            // Build rich display value from semantic fields
            const displayValue = buildCrtpDisplayValue(event.packet)
            const previous = variables[channelKey] ?? createVariableEntry(channelKey, event.timestampMs, runtimeDeviceRef(session, state.config), 'protocol-text')

            if (!variablesChanged) {
              variables = { ...variables }
              variablesChanged = true
            }

            variables[channelKey] = {
              ...previous,
              name: channelKey,
              sourceKind: 'protocol-text',
              currentValue: displayValue,
              parserName: 'crtp',
              sampleCount: previous.sampleCount + 1,
              updatedAtMs: event.timestampMs,
            }

            // Build rich console text from semantic fields
            const consoleText = buildCrtpConsoleText(event.packet)
            pendingConsoleEntries.push(asConsoleEntry({
              kind: 'line',
              timestampMs: event.timestampMs,
              source: event.source,
              line: { direction: 'rx', text: consoleText, raw: [], rawLength: event.packet.payloadLen },
              parser: { parserName: 'crtp', status: 'parsed', fields: event.packet.fields },
            } as Extract<SerialBusEvent, { kind: 'line' }>))

            continue
          }

          const channelId = event.trigger.channelId
          const previous = variables[channelId] ?? createVariableEntry(channelId, event.timestampMs, runtimeDeviceRef(session, state.config), 'protocol-text')

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
        if (pendingConsoleEntries.length > 0) {
          consoleEntries = appendConsoleHistoryBatch(consoleEntries, pendingConsoleEntries, MAX_CONSOLE_ENTRIES)
          consoleChanged = true
        }
        if (consoleChanged) {
          nextState.consoleEntries = consoleEntries
        }
        if (variablesChanged) {
          nextState.variables = variables
        }

        const nextScriptDefinitions = syncScriptDefinitions(scriptDefinitions, variables)
        if (nextScriptDefinitions !== scriptDefinitions) {
          nextState.scriptDefinitions = nextScriptDefinitions
          scriptDefinitions = nextScriptDefinitions
        }

        if (variablesChanged && state.imuMapMode === 'auto') {
          const nextImuChannelMap = autoDetectImuChannelMap(variables, imuChannelMap)
          if (!isSameImuChannelMap(nextImuChannelMap, imuChannelMap)) {
            imuChannelMap = nextImuChannelMap
            imuChannelMapChanged = true
          }

          const nextImuAttitudeMap = autoDetectImuAttitudeMap(variables, imuAttitudeMap)
          if (!isSameImuAttitudeMap(nextImuAttitudeMap, imuAttitudeMap)) {
            imuAttitudeMap = nextImuAttitudeMap
            imuAttitudeMapChanged = true
          }

          const nextImuQuaternionMap = autoDetectImuQuaternionMap(variables, imuQuaternionMap)
          if (!isSameImuQuaternionMap(nextImuQuaternionMap, imuQuaternionMap)) {
            imuQuaternionMap = nextImuQuaternionMap
            imuQuaternionMapChanged = true
          }
        }
        if (selectedChannelsChanged) {
          nextState.selectedChannels = selectedChannels
        }
        if (imuChannelMapChanged) {
          nextState.imuChannelMap = imuChannelMap
        }
        if (imuAttitudeMapChanged) {
          nextState.imuAttitudeMap = imuAttitudeMap
        }
        if (imuQuaternionMapChanged) {
          nextState.imuQuaternionMap = imuQuaternionMap
        }
        if (shouldRecomputeImuQuality && (variablesChanged || imuChannelMapChanged || imuAttitudeMapChanged || imuQuaternionMapChanged)) {
          const nextImuQuality = computeImuQuality(variables, {
            ...state,
            imuChannelMap,
            imuAttitudeMap,
            imuQuaternionMap,
          })
          if (!isSameImuQualitySnapshot(nextImuQuality, imuQuality)) {
            imuQuality = nextImuQuality
            imuQualityChanged = true
          }
        }
        if (imuQualityChanged) {
          nextState.imuQuality = imuQuality
        }
        if (sessionChanged) {
          nextState.session = session
        }
        if (statusChanged) {
          nextState.status = status
        }
        if (protocolChanged) {
          nextState.protocol = protocol
          const nextRuntimeDevice = createRuntimeDeviceSnapshot(session, state.config, state.ports)
          nextState.runtimeDevice = nextRuntimeDevice

          const telemetryCatalogChanged =
            protocol.schema !== state.protocol.schema ||
            protocol.parserName !== state.protocol.parserName ||
            protocol.lastSchemaAtMs !== state.protocol.lastSchemaAtMs

          const deviceCatalogChanged =
            nextRuntimeDevice.id !== state.runtimeDevice.id ||
            nextRuntimeDevice.status !== state.runtimeDevice.status ||
            nextRuntimeDevice.baudRate !== state.runtimeDevice.baudRate ||
            nextRuntimeDevice.portLabel !== state.runtimeDevice.portLabel ||
            nextRuntimeDevice.transportLabel !== state.runtimeDevice.transportLabel ||
            nextRuntimeDevice.connected !== state.runtimeDevice.connected

          if (telemetryCatalogChanged || deviceCatalogChanged) {
            nextState.runtimeCatalog = createRuntimeCatalog(protocol, nextRuntimeDevice)
          }
        }

        return nextState as AppStore
      })
    } finally {
      profileIngestEvents(performance.now() - startedAtMs, {
        eventCount: events.length,
      })
    }
  },
  toggleChannel: (channel) => {
    set((state) => ({
      selectedChannels: state.selectedChannels.includes(channel)
        ? state.selectedChannels.filter((name) => name !== channel)
        : [...state.selectedChannels, channel],
    }))
  },
  setVisualAidEnabled: (channel, key, enabled) => {
    set((state) => {
      const visualAidState = setWaveformVisualAidPreference(state.visualAidState, channel, key, enabled)
      writeWaveformVisualAidState(visualAidState)
      return { visualAidState }
    })
  },
  setImuChannel: (role, channel) => {
    set((state) => ({
      imuMapMode: 'manual',
      imuChannelMap: {
        ...state.imuChannelMap,
        [role]: channel,
      },
    }))
  },
  setImuAttitudeChannel: (role, channel) => {
    set((state) => ({
      imuMapMode: 'manual',
      imuAttitudeMap: {
        ...state.imuAttitudeMap,
        [role]: channel,
      },
    }))
  },
  setImuQuaternionChannel: (role, channel) => {
    set((state) => ({
      imuMapMode: 'manual',
      imuQuaternionMap: {
        ...state.imuQuaternionMap,
        [role]: channel,
      },
    }))
  },
  autoMapImuChannels: () => {
    set((state) => ({
      imuMapMode: 'auto',
      imuChannelMap: autoDetectImuChannelMap(state.variables, createEmptyImuChannelMap()),
      imuAttitudeMap: autoDetectImuAttitudeMap(state.variables, createEmptyImuAttitudeMap()),
      imuQuaternionMap: autoDetectImuQuaternionMap(state.variables, createEmptyImuQuaternionMap()),
    }))
  },
  setImuOrientationSource: (value) => set({ imuOrientationSource: value }),
  calibrateImuFromCurrentState: () => {
    set((state) => ({
      imuCalibration: {
        accelBiasApplied: state.imuChannelMap.accelX !== null,
        gyroBiasApplied: state.imuChannelMap.gyroZ !== null,
        sourceLabel: state.imuOrientationSource,
        lastCalibratedAtMs: Date.now(),
      },
      imuQuality: {
        ...state.imuQuality,
        level: 'good',
        label: 'Calibration stored',
        details: 'Current offsets and orientation source have been recorded.',
        timestampMs: Date.now(),
      },
    }))
  },
  resetImuCalibration: () => {
    set({
      imuCalibration: {
        accelBiasApplied: false,
        gyroBiasApplied: false,
        sourceLabel: null,
        lastCalibratedAtMs: null,
      },
      imuQuality: {
        level: 'warning',
        label: 'Calibration cleared',
        details: 'IMU will use live samples without stored offsets.',
        timestampMs: Date.now(),
      },
    })
  },
  colorForChannel: (channel) => colorForChannel(channel),
}))
