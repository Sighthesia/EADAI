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
  getLogicAnalyzerStatus,
  getMcpServerStatus,
  getSessionSnapshot,
  listSerialPorts,
  refreshLogicAnalyzerDevices,
  sendBmi088Command,
  sendSerial,
  startLogicAnalyzerCapture,
  stopLogicAnalyzerCapture,
} from '../lib/tauri'
import { clampWaveformWindowMs, DEFAULT_WAVEFORM_WINDOW_MS } from '../lib/waveformWindow'
import type {
  AppStatus,
  Bmi088HostCommand,
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
  SerialBusEvent,
  SerialDeviceInfo,
  SessionSnapshot,
  UiAnalysisPayload,
  UiLineDirection,
  UiParserMeta,
  UiProtocolHandshakeEvent,
  UiProtocolSnapshot,
  UiScriptHookExample,
  UiTelemetrySchemaPayload,
  UiTelemetrySamplePayload,
  UiTriggerPayload,
  VariableEntry,
} from '../types'

const MAX_CONSOLE_ENTRIES = 400
const MAX_SAMPLES_PER_CHANNEL = 960
const CHANNEL_COLORS = ['#4FC3F7', '#C792EA', '#F78C6C', '#A5E075', '#E6C07B', '#82AAFF']
const DEFAULT_FAKE_PROFILE = 'telemetry-lab'
const BMI088_PROTOCOL_NAME = 'bmi088_uart4'
const BMI088_PROTOCOL_SCRIPT = `// BMI088 UART4 schema-first telemetry
onSchema((fields, rateHz, sampleLen) => {
  console.log('schema', { rateHz, sampleLen, fields })
})

onSample((record) => {
  console.log('sample', record)
})`
const PROTOCOL_TIMELINE_LIMIT = 64

type AppStore = {
  ports: SerialDeviceInfo[]
  session: SessionSnapshot
  mcp: McpServerStatus
  config: ConnectRequest
  appendNewline: boolean
  commandInput: string
  consoleDisplayMode: 'text' | 'hex' | 'binary'
  consoleEntries: ConsoleEntry[]
  protocol: UiProtocolSnapshot
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
  status: AppStatus
  setCommandInput: (value: string) => void
  setAppendNewline: (value: boolean) => void
  setConsoleDisplayMode: (value: 'text' | 'hex' | 'binary') => void
  setWaveformWindowMs: (value: number | ((current: number) => number)) => void
  patchConfig: (value: Partial<ConnectRequest>) => void
  bootstrap: () => Promise<void>
  refreshMcpStatus: () => Promise<void>
  refreshPorts: () => Promise<void>
  refreshPortsSilently: () => Promise<void>
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  send: () => Promise<void>
  sendBmi088Command: (command: Bmi088HostCommand) => Promise<void>
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
  protocolScript: string
  protocolHookExamples: UiScriptHookExample[]
}

const defaultStatus = (): AppStatus => ({
  tone: 'neutral',
  message: 'Ready. Fake stream will autostart for UI debugging.',
})

const MCP_STATUS_POLL_INTERVAL_MS = 1000
const DEFAULT_LOGIC_SAMPLE_COUNT = 2048

const defaultLogicAnalyzerStatus = (): LogicAnalyzerStatus => ({
  available: false,
  executable: null,
  sessionState: 'idle',
  devices: [],
  selectedDeviceRef: null,
  activeCapture: null,
  lastCapture: null,
  lastScanAtMs: null,
  scanOutput: null,
  lastError: null,
  capturePlan: null,
  linuxFirstNote: 'Linux-first sigrok path; install sigrok-cli or set EADAI_SIGROK_CLI.',
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
  consoleDisplayMode: 'text' as const,
  consoleEntries: [],
  protocol: defaultProtocolSnapshot(),
  protocolScript: BMI088_PROTOCOL_SCRIPT,
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
  status: defaultStatus(),
  setCommandInput: (value) => set({ commandInput: value }),
  setAppendNewline: (value) => set({ appendNewline: value }),
  setConsoleDisplayMode: (value) => set({ consoleDisplayMode: value }),
  setWaveformWindowMs: (value) =>
    set((state) => ({
      waveformWindowMs: clampWaveformWindowMs(typeof value === 'function' ? value(state.waveformWindowMs) : value),
    })),
  patchConfig: (value) => set((state) => ({ config: { ...state.config, ...value } })),
  bootstrap: async () => {
    const [ports, session, mcp, logicAnalyzer] = await Promise.all([
      listSerialPorts(),
      getSessionSnapshot(),
      readMcpStatusWithRetry(),
      refreshLogicAnalyzerDevicesSafely(),
    ])
    set((state) => ({
      ports,
      session,
      mcp,
      logicAnalyzer,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, logicAnalyzer),
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
  refreshLogicAnalyzerStatus: async () => {
    const logicAnalyzer = await getLogicAnalyzerStatus()
    set((state) => ({
      logicAnalyzer,
      logicAnalyzerConfig: syncLogicAnalyzerConfig(state.logicAnalyzerConfig, logicAnalyzer),
    }))
  },
  refreshLogicAnalyzerDevices: async () => {
    const logicAnalyzer = await refreshLogicAnalyzerDevices()
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
    set((state) => buildPortsRefreshState(state, ports, false))
  },
  refreshPortsSilently: async () => {
    const ports = await listSerialPorts()
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
      variables: {},
      selectedChannels: [],
      protocol: defaultProtocolSnapshot(),
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
      status: { tone: 'neutral', message: 'Serial session stopped.' },
      variables: {},
      selectedChannels: [],
      protocol: defaultProtocolSnapshot(),
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
      status: { tone: 'success', message: `Sent ${payload}.` },
    })
  },
  sendBmi088Command: async (command) => {
    await sendBmi088Command({ command })
    set({
      status: { tone: 'success', message: `Sent BMI088 ${command}.` },
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
      let imuChannelMap = state.imuChannelMap
      let imuAttitudeMap = state.imuAttitudeMap
      let imuQuaternionMap = state.imuQuaternionMap
      let imuCalibration = state.imuCalibration
      let imuQuality = state.imuQuality
      let session = state.session
      let status = state.status
      let protocol = state.protocol
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
          consoleEntries = [...consoleEntries, consoleEntry].slice(-MAX_CONSOLE_ENTRIES)
          consoleChanged = true

          protocol = ingestProtocolLine(protocol, event)
          protocolChanged = true

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
            trend: resolveTrendFromValues(previous.numericValue, Number.isFinite(numericValue) ? numericValue : previous.numericValue),
            unit: event.parser.fields.unit ?? previous.unit ?? null,
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

          const nextImuQuality = computeImuQuality(variables, state)
          if (!isSameImuQualitySnapshot(nextImuQuality, imuQuality)) {
            imuQuality = nextImuQuality
            imuQualityChanged = true
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

        if (event.kind === 'telemetrySchema') {
          protocol = ingestProtocolSchema(protocol, event)
          protocolChanged = true

          const nextVariables = { ...state.variables }
          let changed = false

          event.schema.fields.forEach((field, index) => {
            if (!nextVariables[field.name]) {
              nextVariables[field.name] = createVariableEntry(field.name, event.timestampMs)
              changed = true
            }

            nextVariables[field.name] = {
              ...nextVariables[field.name],
              name: field.name,
              unit: field.unit,
              parserName: 'bmi088_schema',
              updatedAtMs: event.timestampMs,
              sampleCount: nextVariables[field.name].sampleCount,
            }

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

        if (event.kind === 'telemetrySample') {
          protocol = ingestProtocolSample(protocol, event)
          protocolChanged = true

          const schemaFields = event.sample.fields
          if (!variablesChanged) {
            variables = { ...variables }
            variablesChanged = true
          }

          for (const field of schemaFields) {
            const previous = variables[field.name] ?? createVariableEntry(field.name, event.timestampMs)
            const nextNumericValue = field.value

            variables[field.name] = {
              ...previous,
              name: field.name,
              currentValue: field.value.toFixed(field.scaleQ < 0 ? 2 : 6),
              previousValue: previous.numericValue,
              numericValue: nextNumericValue,
              trend: resolveTrendFromValues(previous.numericValue, nextNumericValue),
              unit: field.unit ?? previous.unit ?? null,
              parserName: 'bmi088_sample',
              sampleCount: previous.sampleCount + 1,
              updatedAtMs: event.timestampMs,
              points: [...previous.points, { timestampMs: event.timestampMs, value: field.value }].slice(-MAX_SAMPLES_PER_CHANNEL),
            }
          }

          const nextSelectedChannels = autoSelectChannels(selectedChannels, schemaFields[0]?.name ?? 'acc_x')
          if (nextSelectedChannels !== selectedChannels) {
            selectedChannels = nextSelectedChannels
            selectedChannelsChanged = true
          }

          const nextImuQuality = computeImuQuality(variables, state)
          if (!isSameImuQualitySnapshot(nextImuQuality, imuQuality)) {
            imuQuality = nextImuQuality
            imuQualityChanged = true
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

        if (state.imuMapMode === 'auto') {
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
      }

      return nextState as AppStore
    })
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

const asConsoleEntry = (event: Extract<SerialBusEvent, { kind: 'line' }>): ConsoleEntry => ({
  id: `${event.timestampMs}-${event.line.direction}-${event.line.text}`,
  direction: event.line.direction,
  text: event.line.text,
  timestampMs: event.timestampMs,
  raw: event.line.raw,
  parser: event.parser,
})

const createVariableEntry = (channelId: string, updatedAtMs: number): VariableEntry => ({
  name: channelId,
  currentValue: '—',
  trend: 'flat',
  unit: null,
  parserName: null,
  sampleCount: 0,
  updatedAtMs,
  points: [],
  analysis: null,
  latestTrigger: null,
  triggerCount: 0,
})

function createProtocolHookExamples(): UiScriptHookExample[] {
  return [
    {
      name: 'Schema callback',
      snippet: `onSchema((fields, rateHz, sampleLen) => {
  console.log('BMI088 schema', { rateHz, sampleLen, fields })
})`,
    },
    {
      name: 'Sample callback',
      snippet: `onSample((record) => {
  const { roll, pitch, yaw } = record
  console.log('BMI088 sample', { roll, pitch, yaw })
})`,
    },
  ]
}

function defaultProtocolSnapshot(): UiProtocolSnapshot {
  return {
    active: false,
    parserName: BMI088_PROTOCOL_NAME,
    transportLabel: 'BMI088 UART4',
    baudRate: 115200,
    phase: 'stopped',
    schema: null,
    lastPacketKind: null,
    lastPacketRawFrame: null,
    lastSchemaAtMs: null,
    lastSampleAtMs: null,
    lastHandshakeAtMs: null,
    timeline: [],
  }
}

const ingestProtocolConnection = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'connection' }>,
): UiProtocolSnapshot => {
  const nextPhase =
    event.connection.state === 'connected'
      ? 'awaitingSchema'
      : event.connection.state === 'stopped'
        ? 'stopped'
        : protocol.phase

  const timeline = appendProtocolTimeline(protocol.timeline, {
    timestampMs: event.timestampMs,
    direction: 'rx',
    command: event.connection.state === 'stopped' ? 'STOP' : 'REQ_SCHEMA',
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

const ingestProtocolLine = (
  protocol: UiProtocolSnapshot,
  event: Extract<SerialBusEvent, { kind: 'line' }>,
): UiProtocolSnapshot => {
  const parserName = event.parser.parserName ?? protocol.parserName
  const command = protocolCommandFromLine(event.line.text)
  const isHandshake = parserName === 'bmi088_command' || command !== null

  if (!isHandshake) {
    return protocol
  }

  const nextPhase = command === 'ACK' ? 'awaitingStart' : command === 'START' ? 'streaming' : command === 'STOP' ? 'stopped' : protocol.phase

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

const ingestProtocolSchema = (
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

const ingestProtocolSample = (
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

const appendProtocolTimeline = (timeline: UiProtocolHandshakeEvent[], next: UiProtocolHandshakeEvent) =>
  [...timeline, next].slice(-PROTOCOL_TIMELINE_LIMIT)

const protocolCommandFromLine = (text: string): UiProtocolHandshakeEvent['command'] | null => {
  const normalized = text.trim().toUpperCase()
  if (normalized === 'ACK' || normalized === 'START' || normalized === 'STOP' || normalized === 'REQ_SCHEMA') {
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
    default:
      return command
  }
}

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
  if (selectedChannels.length > 0) {
    return selectedChannels
  }
  return [channelId]
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

const resolveTrendFromValues = (previousValue: number | undefined, nextValue: number | undefined) => {
  if (previousValue === undefined || previousValue === null || nextValue === undefined || nextValue === null) {
    return 'flat' as const
  }

  return trendFromDelta(nextValue - previousValue)
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

const readLogicAnalyzerStatusSafely = async () => {
  try {
    return await getLogicAnalyzerStatus()
  } catch {
    return defaultLogicAnalyzerStatus()
  }
}

const refreshLogicAnalyzerDevicesSafely = async () => {
  try {
    return await refreshLogicAnalyzerDevices()
  } catch {
    return readLogicAnalyzerStatusSafely()
  }
}

const buildPortsRefreshState = (
  state: AppStore,
  ports: SerialDeviceInfo[],
  silent: boolean,
): Partial<AppStore> => {
  const nextState: Partial<AppStore> = {
    ports,
    config: {
      ...state.config,
      port:
        state.config.sourceKind === 'fake'
          ? state.config.port
          : ports.some((port) => port.portName === state.config.port)
            ? state.config.port
            : ports[0]?.portName ?? '',
    },
  }

  if (!silent) {
    nextState.status = {
      tone: 'neutral',
      message: ports.length > 0 ? `Found ${ports.length} serial devices.` : 'No serial devices found.',
    }
    return nextState
  }

  if (sameSerialDeviceList(state.ports, ports)) {
    return nextState
  }

  nextState.status = {
    tone: 'neutral',
    message: ports.length > 0 ? `Detected serial device change. ${ports.length} available.` : 'Detected serial device removal.',
  }
  return nextState
}

const sameSerialDeviceList = (left: SerialDeviceInfo[], right: SerialDeviceInfo[]) =>
  left.length === right.length &&
  left.every((device, index) => {
    const next = right[index]
    return (
      device.portName === next?.portName &&
      device.displayName === next.displayName &&
      device.portType === next.portType &&
      device.manufacturer === next.manufacturer &&
      device.product === next.product &&
      device.serialNumber === next.serialNumber &&
      device.vid === next.vid &&
      device.pid === next.pid
    )
  })

const syncLogicAnalyzerConfig = (
  config: LogicAnalyzerConfig,
  logicAnalyzer: LogicAnalyzerStatus,
): LogicAnalyzerConfig => {
  const availableDeviceRefs = new Set(logicAnalyzer.devices.map((device) => device.reference))
  const nextDeviceRef =
    config.deviceRef && availableDeviceRefs.has(config.deviceRef)
      ? config.deviceRef
      : logicAnalyzer.selectedDeviceRef ?? logicAnalyzer.devices[0]?.reference ?? ''

  const availableChannelLabels = new Set(logicAnalyzer.lastCapture?.channels.map((channel) => channel.label) ?? [])
  const nextChannels = config.selectedChannelLabels.filter((channel) => availableChannelLabels.has(channel))

  return {
    ...config,
    deviceRef: nextDeviceRef,
    selectedChannelLabels:
      nextChannels.length > 0
        ? nextChannels
        : logicAnalyzer.lastCapture?.channels.map((channel) => channel.label) ?? config.selectedChannelLabels,
  }
}

const parseLogicSamplerate = (value: string) => {
  const parsed = Number(value.trim())
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null
}

export const shouldPollMcpStatus = (mcp: McpServerStatus) => !mcp.isRunning && !mcp.lastError

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

const isSameImuChannelMap = (left: ImuChannelMap, right: ImuChannelMap) =>
  Object.keys(left).every((key) => left[key as ImuChannelRole] === right[key as ImuChannelRole])

const isSameImuAttitudeMap = (left: ImuAttitudeMap, right: ImuAttitudeMap) =>
  Object.keys(left).every((key) => left[key as ImuAttitudeRole] === right[key as ImuAttitudeRole])

const isSameImuQuaternionMap = (left: ImuQuaternionMap, right: ImuQuaternionMap) =>
  Object.keys(left).every((key) => left[key as ImuQuaternionRole] === right[key as ImuQuaternionRole])

const isSameImuQualitySnapshot = (left: ImuQualitySnapshot, right: ImuQualitySnapshot) =>
  left.level === right.level && left.label === right.label && left.details === right.details && left.timestampMs === right.timestampMs

const computeImuQuality = (variables: Record<string, VariableEntry>, state: AppStore) => {
  const imuRawCount = ['accelX', 'accelY', 'accelZ', 'gyroX', 'gyroY', 'gyroZ'].filter((role) => {
    const key = role as keyof typeof state.imuChannelMap
    return Boolean(state.imuChannelMap[key])
  }).length
  const quaternionCount = ['quatW', 'quatX', 'quatY', 'quatZ'].filter((role) => {
    const key = role as keyof typeof state.imuQuaternionMap
    return Boolean(state.imuQuaternionMap[key])
  }).length

  if (state.imuOrientationSource === 'rawFusion') {
    if (quaternionCount === 4) {
      return {
        level: 'good' as const,
        label: 'Rust fused quaternion active',
        details: 'Quaternion output is available and preferred over local approximation.',
        timestampMs: latestUpdateForChannels(variables, Object.values(state.imuQuaternionMap)),
      }
    }
    if (imuRawCount >= 4) {
      return {
        level: 'warning' as const,
        label: 'Local fallback active',
        details: 'Using accel/gyro approximation because fused quaternion is not mapped.',
        timestampMs: latestUpdateForChannels(variables, Object.values(state.imuChannelMap)),
      }
    }
    return {
      level: 'critical' as const,
      label: 'IMU mapping incomplete',
      details: 'Not enough accel/gyro channels are mapped to compute orientation.',
      timestampMs: null,
    }
  }

  if (state.imuOrientationSource === 'directQuaternion') {
    return {
      level: quaternionCount === 4 ? ('good' as const) : ('warning' as const),
      label: quaternionCount === 4 ? 'Quaternion mapped' : 'Quaternion incomplete',
      details:
        quaternionCount === 4
          ? 'Direct quaternion mode is fully mapped.'
          : 'Map W/X/Y/Z to use direct quaternion mode.',
      timestampMs: latestUpdateForChannels(variables, Object.values(state.imuQuaternionMap)),
    }
  }

  const attitudeCount = ['roll', 'pitch', 'yaw'].filter((role) => {
    const key = role as keyof typeof state.imuAttitudeMap
    return Boolean(state.imuAttitudeMap[key])
  }).length
  return {
    level: attitudeCount === 3 ? ('good' as const) : ('warning' as const),
    label: attitudeCount === 3 ? 'Angles mapped' : 'Angles incomplete',
    details:
      attitudeCount === 3
        ? 'Direct angle mode is fully mapped.'
        : 'Map Roll/Pitch/Yaw to use direct angle mode.',
    timestampMs: latestUpdateForChannels(variables, Object.values(state.imuAttitudeMap)),
  }
}

const latestUpdateForChannels = (variables: Record<string, VariableEntry>, channels: Array<string | null>) =>
  channels
    .filter((channel): channel is string => Boolean(channel))
    .map((channel) => variables[channel]?.updatedAtMs ?? null)
    .reduce<number | null>((latest, timestamp) => {
      if (timestamp === null) {
        return latest
      }
      return latest === null ? timestamp : Math.max(latest, timestamp)
    }, null)
