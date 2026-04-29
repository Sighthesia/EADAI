import type { AppStatus, ConnectRequest, McpServerStatus, SerialDeviceInfo } from '../types'
import { getMcpServerStatus } from '../lib/tauri'

export const readMcpStatusWithRetry = async (): Promise<McpServerStatus> => {
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

interface PortsRefreshResult {
  ports: SerialDeviceInfo[]
  config: ConnectRequest
  status?: AppStatus
}

export const buildPortsRefreshState = (
  state: { config: ConnectRequest; ports: SerialDeviceInfo[] },
  ports: SerialDeviceInfo[],
  silent: boolean,
): PortsRefreshResult => {
  const result: PortsRefreshResult = {
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
    result.status = {
      tone: 'neutral',
      message: ports.length > 0 ? `Found ${ports.length} serial devices.` : 'No serial devices found.',
    }
    return result
  }

  if (sameSerialDeviceList(state.ports, ports)) {
    return result
  }

  result.status = {
    tone: 'neutral',
    message: ports.length > 0 ? `Detected serial device change. ${ports.length} available.` : 'Detected serial device removal.',
  }
  return result
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
