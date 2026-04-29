import type {
  ConnectRequest,
  SerialDeviceInfo,
  SessionSnapshot,
  UiProtocolSnapshot,
  UiRuntimeCatalogSnapshot,
  UiRuntimeDeviceSnapshot,
  UiRuntimeTelemetryCatalogField,
  UiRuntimeTelemetryCatalogSnapshot,
} from '../types'
import { DEFAULT_FAKE_PROFILE, RUNTIME_COMMAND_CATALOG } from './constants'

export function createRuntimeCatalog(protocol: UiProtocolSnapshot, runtimeDevice: UiRuntimeDeviceSnapshot): UiRuntimeCatalogSnapshot {
  return {
    device: runtimeDevice,
    commands: RUNTIME_COMMAND_CATALOG.map((item) => ({ ...item })),
    telemetry: createRuntimeTelemetryCatalog(protocol),
  }
}

export function createRuntimeDeviceSnapshot(
  session: SessionSnapshot,
  config: ConnectRequest,
  ports: SerialDeviceInfo[],
): UiRuntimeDeviceSnapshot {
  const sourceKind = session.transport === 'serial' || session.transport === 'fake' ? session.transport : config.sourceKind
  const portLabel = sourceKind === 'fake' ? `fake://${config.fakeProfile ?? DEFAULT_FAKE_PROFILE}` : session.port ?? config.port ?? null
  const serialDevice = sourceKind === 'serial' && portLabel ? ports.find((port) => port.portName === portLabel) ?? null : null
  const label = sourceKind === 'fake' ? fakeProfileLabel(config.fakeProfile ?? DEFAULT_FAKE_PROFILE) : serialDevice?.displayName ?? portLabel ?? 'Serial device'
  const detail = sourceKind === 'fake' ? 'Built-in development telemetry source' : formatSerialDeviceDetail(serialDevice, portLabel)

  return {
    id: portLabel ?? sourceKind,
    label,
    detail,
    status: session.isRunning ? session.connectionState ?? 'connected' : 'Idle',
    transportLabel: sourceKind === 'fake' ? 'Fake stream' : 'Serial device',
    portLabel,
    baudRate: session.baudRate ?? config.baudRate,
    sourceKind,
    connected: session.isRunning,
    serialDevice,
  }
}

export function runtimeDeviceRef(session: SessionSnapshot, config: ConnectRequest) {
  if (session.transport === 'fake') {
    return `fake://${config.fakeProfile ?? DEFAULT_FAKE_PROFILE}`
  }

  return session.port ?? config.port ?? null
}

function fakeProfileLabel(profile: string) {
  switch (profile) {
    case 'noisy-monitor':
      return 'Noisy Monitor'
    case 'imu-lab':
      return 'IMU Lab'
    default:
      return 'Telemetry Lab'
  }
}

function formatSerialDeviceDetail(device: SerialDeviceInfo | null, fallbackPort: string | null) {
  if (!device) {
    return fallbackPort ? `Serial port ${fallbackPort}` : 'Awaiting serial device details'
  }

  const parts = [device.product, device.manufacturer, formatUsbIdentifier(device)].filter(Boolean)
  return parts.length > 0 ? parts.join(' · ') : device.displayName
}

function formatUsbIdentifier(device: SerialDeviceInfo) {
  if (device.vid == null || device.pid == null) {
    return null
  }

  return `USB ${formatUsbHex(device.vid)}:${formatUsbHex(device.pid)}`
}

function formatUsbHex(value: number) {
  return value.toString(16).padStart(4, '0').toUpperCase()
}

function createRuntimeTelemetryCatalog(protocol: UiProtocolSnapshot): UiRuntimeTelemetryCatalogSnapshot {
  return {
    parserName: protocol.parserName,
    rateHz: protocol.schema?.rateHz ?? null,
    sampleLen: protocol.schema?.sampleLen ?? null,
    fields: protocol.schema?.fields.map((field, index) => createRuntimeTelemetryCatalogField(field.name, field.unit, field.scaleQ, index)) ?? [],
    lastSchemaAtMs: protocol.lastSchemaAtMs ?? null,
    lastSampleAtMs: protocol.lastSampleAtMs ?? null,
  }
}

function createRuntimeTelemetryCatalogField(
  name: string,
  unit: string,
  scaleQ: number,
  index: number,
): UiRuntimeTelemetryCatalogField {
  return {
    name,
    unit,
    scaleQ,
    index,
  }
}
