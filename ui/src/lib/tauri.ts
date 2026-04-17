import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ConnectRequest,
  LogicAnalyzerCaptureRequest,
  LogicAnalyzerStatus,
  McpServerStatus,
  SendRequest,
  SerialBusEvent,
  SerialDeviceInfo,
  SessionSnapshot,
} from '../types'

const SERIAL_EVENT_NAME = 'serial-bus-event'
const SERIAL_DEVICE_EVENT_NAME = 'serial-devices-changed'

export const listSerialPorts = () => invoke<SerialDeviceInfo[]>('list_serial_ports')

export const getSessionSnapshot = () => invoke<SessionSnapshot>('get_session_snapshot')

export const getMcpServerStatus = () => invoke<McpServerStatus>('get_mcp_server_status')

export const getLogicAnalyzerStatus = () => invoke<LogicAnalyzerStatus>('get_logic_analyzer_status')

export const refreshLogicAnalyzerDevices = () =>
  invoke<LogicAnalyzerStatus>('refresh_logic_analyzer_devices')

export const startLogicAnalyzerCapture = (request: LogicAnalyzerCaptureRequest) =>
  invoke<LogicAnalyzerStatus>('start_logic_analyzer_capture', { request })

export const stopLogicAnalyzerCapture = () => invoke<LogicAnalyzerStatus>('stop_logic_analyzer_capture')

export const connectSerial = (request: ConnectRequest) =>
  invoke<SessionSnapshot>('connect_serial', { request })

export const disconnectSerial = () => invoke<SessionSnapshot>('disconnect_serial')

export const sendSerial = (request: SendRequest) => invoke<void>('send_serial', { request })

export const listenSerialBus = async (
  onMessage: (event: SerialBusEvent) => void,
): Promise<UnlistenFn> =>
  listen<SerialBusEvent>(SERIAL_EVENT_NAME, (event) => {
    onMessage(event.payload)
  })

export const listenSerialDevicesChanged = async (
  onChange: () => void,
): Promise<UnlistenFn> =>
  listen(SERIAL_DEVICE_EVENT_NAME, () => {
    onChange()
  })
