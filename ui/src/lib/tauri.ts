import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  ConnectRequest,
  McpServerStatus,
  SendRequest,
  SerialBusEvent,
  SessionSnapshot,
} from '../types'

const SERIAL_EVENT_NAME = 'serial-bus-event'

export const listSerialPorts = () => invoke<string[]>('list_serial_ports')

export const getSessionSnapshot = () => invoke<SessionSnapshot>('get_session_snapshot')

export const getMcpServerStatus = () => invoke<McpServerStatus>('get_mcp_server_status')

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
