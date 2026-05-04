import { useEffect, useMemo, useState } from 'react'
import { useAppStore } from '../store/appStore'
import type { ConnectRequest, LogicAnalyzerConfig, LogicAnalyzerStatus, SerialDeviceInfo, SessionSnapshot, SourceKind } from '../types'

const FAKE_PROFILES = [
  { value: 'telemetry-lab', label: 'Telemetry Lab' },
  { value: 'noisy-monitor', label: 'Noisy Monitor' },
  { value: 'imu-lab', label: 'IMU Lab' },
]

const PARSER_OPTIONS = [
  { value: 'auto', label: 'Auto' },
  { value: 'bmi088', label: 'BMI088' },
  { value: 'mavlink', label: 'MAVLink' },
  { value: 'crtp', label: 'CRTP' },
  { value: 'key_value', label: 'Key Value' },
  { value: 'measurements', label: 'Measurements' },
] as const

const DEVICE_META_STORAGE_KEY = 'eadai.connection.device-meta.v1'

type DeviceCardKind = 'serial' | 'logic'

type DeviceCard = {
  id: string
  kind: DeviceCardKind
  sourceKind?: SourceKind
  port?: string
  fakeProfile?: string
  serialDevice?: SerialDeviceInfo
  title: string
  subtitle: string
  status: string
  metric: string
  info: string
  active: boolean
  connected: boolean
  editableName: string
  note: string
}

type DeviceMetaMap = Record<string, { alias: string; note: string }>

type ContextMenuState = {
  deviceId: string
  x: number
  y: number
}

export function ConnectionPanel() {
  const ports = useAppStore((state) => state.ports)
  const config = useAppStore((state) => state.config)
  const session = useAppStore((state) => state.session)
  const logicAnalyzer = useAppStore((state) => state.logicAnalyzer)
  const logicAnalyzerConfig = useAppStore((state) => state.logicAnalyzerConfig)
  const connect = useAppStore((state) => state.connect)
  const disconnect = useAppStore((state) => state.disconnect)
  const refreshPorts = useAppStore((state) => state.refreshPorts)
  const refreshLogicAnalyzerDevices = useAppStore((state) => state.refreshLogicAnalyzerDevices)
  const patchLogicAnalyzerConfig = useAppStore((state) => state.patchLogicAnalyzerConfig)
  const startLogicAnalyzerCapture = useAppStore((state) => state.startLogicAnalyzerCapture)
  const stopLogicAnalyzerCapture = useAppStore((state) => state.stopLogicAnalyzerCapture)
  const patchConfig = useAppStore((state) => state.patchConfig)
  const colorForChannel = useAppStore((state) => state.colorForChannel)

  const [deviceMeta, setDeviceMeta] = useState<DeviceMetaMap>(() => readDeviceMeta())
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('')
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)
  const [draftAlias, setDraftAlias] = useState('')
  const [draftNote, setDraftNote] = useState('')

  const logicCaptureActive = logicAnalyzer.sessionState === 'capturing'
  const isDev = import.meta.env.DEV

  const deviceCards = useMemo<DeviceCard[]>(() => {
    const serialPortMap = new Map(ports.map((port) => [port.portName, port]))
    if (config.sourceKind === 'serial' && config.port && !serialPortMap.has(config.port)) {
      serialPortMap.set(config.port, buildFallbackSerialDevice(config.port))
    }
    if (session.transport === 'serial' && session.port && !serialPortMap.has(session.port)) {
      serialPortMap.set(session.port, buildFallbackSerialDevice(session.port))
    }

    const serialCards = Array.from(serialPortMap.values())
      .sort(compareSerialDevices)
      .map((port) => {
        const portName = port.portName
        const meta = deviceMeta[portName]
        const title = meta?.alias.trim() || port.displayName || portName
        const connected = session.transport === 'serial' && session.port === portName && session.isRunning
        return {
          id: portName,
          kind: 'serial' as const,
          sourceKind: 'serial' as const,
          port: portName,
          serialDevice: port,
          title,
          subtitle: portName,
          status: connected ? session.connectionState ?? 'connected' : config.port === portName ? 'armed' : 'idle',
          metric: `${session.baudRate ?? config.baudRate} baud`,
          info: meta?.note.trim() || formatSerialDeviceInfo(port),
          active: selectedDeviceId === portName,
          connected,
          editableName: title,
          note: meta?.note ?? '',
        }
      })

    const fakeCards = isDev
      ? FAKE_PROFILES.map((profile) => {
          const fakeId = `fake://${profile.value}`
          const meta = deviceMeta[fakeId]
          const title = meta?.alias.trim() || profile.label
          const connected = session.transport === 'fake' && session.port === fakeId && session.isRunning
          return {
            id: fakeId,
            kind: 'serial' as const,
            sourceKind: 'fake' as const,
            port: fakeId,
            fakeProfile: profile.value,
            title,
            subtitle: fakeId,
            status: connected ? session.connectionState ?? 'connected' : config.fakeProfile === profile.value ? 'armed' : 'idle',
            metric: `${config.baudRate} baud`,
            info: meta?.note.trim() || 'Built-in development telemetry source',
            active: selectedDeviceId === fakeId,
            connected,
            editableName: title,
            note: meta?.note ?? '',
          }
        })
      : []

    const logicCards = logicAnalyzer.devices.map((device) => {
      const meta = deviceMeta[device.reference]
      const title = meta?.alias.trim() || device.name
      return {
        id: device.reference,
        kind: 'logic' as const,
        title,
        subtitle: device.reference,
        status: logicAnalyzer.activeCapture?.outputPath && logicAnalyzer.selectedDeviceRef === device.reference ? 'capturing' : logicAnalyzer.selectedDeviceRef === device.reference ? logicAnalyzer.sessionState : 'ready',
        metric: `${device.channels.length || 0} channels`,
        info: meta?.note.trim() || device.note || device.driver || 'sigrok logic analyzer',
        active: selectedDeviceId === device.reference,
        connected: logicAnalyzer.selectedDeviceRef === device.reference,
        editableName: title,
        note: meta?.note ?? '',
      }
    })

    return [...serialCards, ...fakeCards, ...logicCards]
  }, [config.baudRate, config.fakeProfile, config.port, config.sourceKind, deviceMeta, isDev, logicAnalyzer.activeCapture?.outputPath, logicAnalyzer.devices, logicAnalyzer.selectedDeviceRef, logicAnalyzer.sessionState, ports, selectedDeviceId, session.baudRate, session.connectionState, session.isRunning, session.port, session.transport])

  const selectedCard = deviceCards.find((card) => card.id === selectedDeviceId) ?? deviceCards[0]

  useEffect(() => {
    window.localStorage.setItem(DEVICE_META_STORAGE_KEY, JSON.stringify(deviceMeta))
  }, [deviceMeta])

  useEffect(() => {
    if (deviceCards.length === 0) {
      if (selectedDeviceId !== '') {
        setSelectedDeviceId('')
      }
      return
    }

    if (!selectedDeviceId || !deviceCards.some((card) => card.id === selectedDeviceId)) {
      setSelectedDeviceId(deviceCards[0].id)
    }
  }, [deviceCards, selectedDeviceId])

  useEffect(() => {
    if (!contextMenu) {
      return
    }

    const closeMenu = () => setContextMenu(null)
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setContextMenu(null)
      }
    }

    window.addEventListener('pointerdown', closeMenu)
    window.addEventListener('scroll', closeMenu, true)
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      window.removeEventListener('pointerdown', closeMenu)
      window.removeEventListener('scroll', closeMenu, true)
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [contextMenu])

  const openEditMenu = (deviceId: string, x: number, y: number) => {
    const meta = deviceMeta[deviceId]
    const fallback = deviceCards.find((card) => card.id === deviceId)
    setDraftAlias(meta?.alias ?? fallback?.editableName ?? '')
    setDraftNote(meta?.note ?? '')
    setContextMenu({ deviceId, x, y })
  }

  const saveDeviceMeta = () => {
    if (!contextMenu) {
      return
    }

    setDeviceMeta((current) => ({
      ...current,
      [contextMenu.deviceId]: {
        alias: draftAlias,
        note: draftNote,
      },
    }))
    setContextMenu(null)
  }

  const clearDeviceMeta = () => {
    if (!contextMenu) {
      return
    }

    setDeviceMeta((current) => {
      const next = { ...current }
      delete next[contextMenu.deviceId]
      return next
    })
    setContextMenu(null)
  }

  return (
    <section className="panel connection-panel">
      <div className="connection-layout">
        <section className="connection-device-deck">
          <div className="variables-list connection-device-list">
            {deviceCards.map((card, index) => (
              <article
                key={card.id}
                className={`variable-card connection-device-card ${card.active ? 'selected' : ''} ${card.connected ? 'device-connected' : ''}`}
                role="button"
                tabIndex={0}
                onClick={() => {
                  setSelectedDeviceId(card.id)
                  if (card.kind === 'logic') {
                    patchLogicAnalyzerConfig({ deviceRef: card.id })
                  } else {
                    patchConfig({
                      sourceKind: card.sourceKind ?? 'serial',
                      port: card.port ?? '',
                      fakeProfile: card.sourceKind === 'fake' ? card.fakeProfile ?? null : null,
                      parser: config.parser ?? 'auto',
                    })
                  }
                }}
                onContextMenu={(event) => {
                  event.preventDefault()
                  setSelectedDeviceId(card.id)
                  openEditMenu(card.id, event.clientX, event.clientY)
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault()
                    setSelectedDeviceId(card.id)
                  }
                }}
              >
                <span
                  className={`variable-selection-bar ${card.connected ? 'active' : ''}`}
                  style={{ backgroundColor: card.connected ? colorForChannel(`${card.kind}-${index}`) : undefined }}
                />
                <span className="variable-color" style={{ backgroundColor: colorForChannel(`${card.kind}-${index}`) }} />
                <div className="variable-main connection-device-main">
                  <div className="variable-title-row">
                    <strong>{card.title}</strong>
                    <div className="variable-role-chip-row">
                      <span className={`variable-role-chip role-${card.kind === 'logic' ? 'quaternion' : 'raw'}`}>
                        {card.kind === 'logic' ? 'Logic' : 'Serial'}
                      </span>
                      {card.connected ? <span className="metric-chip">Connected</span> : null}
                    </div>
                  </div>
                  <div className="variable-subline">
                    <small>{card.subtitle}</small>
                    <span className="metric-chip">{card.status}</span>
                    <span className="metric-chip">{card.metric}</span>
                  </div>
                  <small className="connection-device-note">{card.info}</small>
                </div>
              </article>
            ))}
          </div>
        </section>

        <section className="connection-detail-shell">
          {!selectedCard ? (
            <section className="connection-section-card">
              <div className="connection-section-header">
                <div>
                  <strong>Connection Details</strong>
                  <small>Select a device card to configure it</small>
                </div>
              </div>
            </section>
          ) : selectedCard.kind === 'logic' ? (
            <LogicConnectionDetail
              logicAnalyzerConfig={logicAnalyzerConfig}
              logicAnalyzer={logicAnalyzer}
              logicCaptureActive={logicCaptureActive}
              patchLogicAnalyzerConfig={patchLogicAnalyzerConfig}
              refreshLogicAnalyzerDevices={refreshLogicAnalyzerDevices}
              startLogicAnalyzerCapture={startLogicAnalyzerCapture}
              stopLogicAnalyzerCapture={stopLogicAnalyzerCapture}
            />
          ) : (
            <SerialConnectionDetail
              config={config}
              session={session}
              selectedCard={selectedCard}
              refreshPorts={refreshPorts}
              patchConfig={patchConfig}
              connect={connect}
              disconnect={disconnect}
            />
          )}
        </section>
      </div>

      {contextMenu ? (
        <div
          className="variable-context-menu"
          style={{ left: clampMenuX(contextMenu.x), top: clampMenuY(contextMenu.y) }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <div className="variable-context-header">
            <strong>{selectedCard?.title ?? contextMenu.deviceId}</strong>
            <small>Edit device alias and note</small>
          </div>
          <label className="connection-context-field">
            <span>Alias</span>
            <input value={draftAlias} onChange={(event) => setDraftAlias(event.target.value)} placeholder="Friendly device name" />
          </label>
          <label className="connection-context-field">
            <span>Note</span>
            <textarea value={draftNote} onChange={(event) => setDraftNote(event.target.value)} rows={3} placeholder="Rack location, probe note, board role..." />
          </label>
          <div className="variable-context-actions">
            <button type="button" className="variable-context-action" onClick={saveDeviceMeta}>
              Save
            </button>
            <button type="button" className="variable-context-action" onClick={() => setContextMenu(null)}>
              Cancel
            </button>
          </div>
          <button type="button" className="variable-context-clear" onClick={clearDeviceMeta}>
            Clear custom device info
          </button>
        </div>
      ) : null}
    </section>
  )
}

function SerialConnectionDetail({
  config,
  session,
  selectedCard,
  refreshPorts,
  patchConfig,
  connect,
  disconnect,
}: {
  config: ConnectRequest
  session: SessionSnapshot
  selectedCard?: DeviceCard
  refreshPorts: () => Promise<void>
  patchConfig: (value: Partial<ConnectRequest>) => void
  connect: () => Promise<void>
  disconnect: () => Promise<void>
}) {
  const handleConnect = async () => {
    if (selectedCard?.sourceKind === 'fake') {
      patchConfig({
        sourceKind: 'fake',
        port: selectedCard.port ?? selectedCard.id,
        fakeProfile: selectedCard.fakeProfile ?? null,
        parser: config.parser ?? 'auto',
      })
    } else if (selectedCard?.port) {
      patchConfig({
        sourceKind: 'serial',
        port: selectedCard.port,
        fakeProfile: null,
        parser: config.parser ?? 'auto',
      })
    }

    await connect()
  }

  return (
    <section className="connection-section-card">
      <div className="connection-section-header">
        <div>
          <strong>Connection Details</strong>
          <small>{selectedCard?.sourceKind === 'fake' ? 'Selected development stream' : 'Selected serial transport'}</small>
        </div>
      </div>
      <div className="panel-grid">
        <label>
          <span>Device</span>
          <input value={selectedCard?.title ?? (config.sourceKind === 'fake' ? 'Fake Stream' : config.port || 'No serial device selected')} readOnly />
        </label>
        <label>
          <span>Identifier</span>
          <input value={selectedCard?.subtitle ?? config.port ?? ''} readOnly />
        </label>
        <label>
          <span>Baud</span>
          <input type="number" value={config.baudRate} onChange={(event) => patchConfig({ baudRate: Number(event.target.value) || 115200 })} />
        </label>
        <label>
          <span>Retry ms</span>
          <input type="number" value={config.retryMs} onChange={(event) => patchConfig({ retryMs: Number(event.target.value) || 1000 })} />
        </label>
        <label>
          <span>Read timeout ms</span>
          <input type="number" value={config.readTimeoutMs} onChange={(event) => patchConfig({ readTimeoutMs: Number(event.target.value) || 50 })} />
        </label>
        <label>
          <span>Parser</span>
          <select value={config.parser ?? 'auto'} onChange={(event) => patchConfig({ parser: event.target.value as ConnectRequest['parser'] })}>
            {PARSER_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="toolbar-row">
        <button className="ghost-button" onClick={() => void refreshPorts()}>
          Refresh Ports
        </button>
        {session.isRunning ? (
          <button className="danger-button" onClick={() => void disconnect()}>
            Disconnect
          </button>
        ) : (
          <button className="primary-button" onClick={() => void handleConnect()}>
            Connect
          </button>
        )}
      </div>
      <dl className="detail-list">
        <div>
          <dt>Status</dt>
          <dd>{session.connectionState ?? 'stopped'}</dd>
        </div>
        <div>
          <dt>Transport</dt>
          <dd>{selectedCard?.sourceKind === 'fake' ? 'Fake Stream' : 'Serial Port'}</dd>
        </div>
        <div>
          <dt>Port Type</dt>
          <dd>{formatPortTypeLabel(selectedCard?.serialDevice?.portType)}</dd>
        </div>
        <div>
          <dt>Baud</dt>
          <dd>{session.baudRate ?? '-'}</dd>
        </div>
        <div>
          <dt>Product</dt>
          <dd>{selectedCard?.serialDevice?.product ?? '-'}</dd>
        </div>
        <div>
          <dt>Manufacturer</dt>
          <dd>{selectedCard?.serialDevice?.manufacturer ?? '-'}</dd>
        </div>
        <div>
          <dt>VID:PID</dt>
          <dd>{formatUsbIdentifier(selectedCard?.serialDevice) ?? '-'}</dd>
        </div>
      </dl>
    </section>
  )
}

function LogicConnectionDetail({
  logicAnalyzerConfig,
  logicAnalyzer,
  logicCaptureActive,
  patchLogicAnalyzerConfig,
  refreshLogicAnalyzerDevices,
  startLogicAnalyzerCapture,
  stopLogicAnalyzerCapture,
}: {
  logicAnalyzerConfig: LogicAnalyzerConfig
  logicAnalyzer: LogicAnalyzerStatus
  logicCaptureActive: boolean
  patchLogicAnalyzerConfig: (value: Partial<LogicAnalyzerConfig>) => void
  refreshLogicAnalyzerDevices: () => Promise<void>
  startLogicAnalyzerCapture: () => Promise<void>
  stopLogicAnalyzerCapture: () => Promise<void>
}) {
  const backendReadyLabel = logicAnalyzer.executable === 'dev-simulator' ? 'dev simulator ready' : logicAnalyzer.available ? 'sigrok ready' : 'sigrok unavailable'

  return (
    <section className="connection-section-card">
      <div className="connection-section-header">
        <div>
          <strong>Connection Details</strong>
          <small>Selected logic analyzer device</small>
        </div>
        <div className={`logic-badge tone-${logicAnalyzer.available ? 'success' : 'warning'}`}>
          {backendReadyLabel}
        </div>
      </div>
      <div className="connection-section-note">{logicAnalyzer.linuxFirstNote}</div>
      <div className="panel-grid">
        <label>
          <span>Device</span>
          <select value={logicAnalyzerConfig.deviceRef} onChange={(event) => patchLogicAnalyzerConfig({ deviceRef: event.target.value })}>
            <option value="">Select a sigrok device</option>
            {logicAnalyzer.devices.map((device) => (
              <option key={device.reference} value={device.reference}>
                {device.name} · {device.reference}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Sample count</span>
          <input type="number" min={1} value={logicAnalyzerConfig.sampleCount} onChange={(event) => patchLogicAnalyzerConfig({ sampleCount: Number(event.target.value) || 2048 })} />
        </label>
        <label>
          <span>Samplerate Hz</span>
          <input value={logicAnalyzerConfig.samplerateHzInput} onChange={(event) => patchLogicAnalyzerConfig({ samplerateHzInput: event.target.value })} placeholder="optional" />
        </label>
        <label>
          <span>Capture state</span>
          <input value={logicAnalyzer.sessionState} readOnly />
        </label>
      </div>
      <div className="toolbar-row">
        <button className="ghost-button" onClick={() => void refreshLogicAnalyzerDevices()}>
          Refresh Devices
        </button>
        {logicCaptureActive ? (
          <button className="danger-button" onClick={() => void stopLogicAnalyzerCapture()}>
            Stop Capture
          </button>
        ) : (
          <button className="primary-button" onClick={() => void startLogicAnalyzerCapture()} disabled={!logicAnalyzerConfig.deviceRef}>
            Start Capture
          </button>
        )}
      </div>
      <dl className="detail-list">
        <div>
          <dt>Executable</dt>
          <dd>{logicAnalyzer.executable ?? '-'}</dd>
        </div>
        <div>
          <dt>Devices</dt>
          <dd>{logicAnalyzer.devices.length}</dd>
        </div>
        <div>
          <dt>Last Capture</dt>
          <dd>{logicAnalyzer.lastCapture ? `${logicAnalyzer.lastCapture.sampleCount} samples` : '-'}</dd>
        </div>
      </dl>
      {logicAnalyzer.lastError ? <div className="logic-banner logic-banner-warning">{logicAnalyzer.lastError}</div> : null}
    </section>
  )
}

function readDeviceMeta(): DeviceMetaMap {
  try {
    const raw = window.localStorage.getItem(DEVICE_META_STORAGE_KEY)
    if (!raw) {
      return {}
    }
    const parsed = JSON.parse(raw)
    return typeof parsed === 'object' && parsed !== null ? parsed : {}
  } catch {
    return {}
  }
}

function buildFallbackSerialDevice(portName: string): SerialDeviceInfo {
  return {
    portName,
    displayName: portName,
    portType: 'unknown',
    manufacturer: null,
    product: null,
    serialNumber: null,
    vid: null,
    pid: null,
  }
}

function compareSerialDevices(left: SerialDeviceInfo, right: SerialDeviceInfo) {
  return (
    rankPortType(left.portType) - rankPortType(right.portType) ||
    left.displayName.localeCompare(right.displayName) ||
    left.portName.localeCompare(right.portName)
  )
}

function rankPortType(portType: SerialDeviceInfo['portType']) {
  switch (portType) {
    case 'usb':
      return 0
    case 'bluetooth':
      return 1
    case 'pci':
      return 2
    case 'unknown':
      return 3
  }
}

function formatSerialDeviceInfo(device: SerialDeviceInfo) {
  const parts = [device.product, device.manufacturer, formatUsbIdentifier(device)].filter(
    (value): value is string => Boolean(value),
  )

  return parts.length > 0 ? parts.join(' · ') : 'Serial telemetry transport'
}

function formatPortTypeLabel(portType?: SerialDeviceInfo['portType']) {
  switch (portType) {
    case 'usb':
      return 'USB serial'
    case 'bluetooth':
      return 'Bluetooth serial'
    case 'pci':
      return 'PCI serial'
    case 'unknown':
      return 'Generic serial'
    default:
      return '-'
  }
}

function formatUsbIdentifier(device?: SerialDeviceInfo) {
  if (device?.vid == null || device.pid == null) {
    return null
  }

  return `${formatUsbHex(device.vid)}:${formatUsbHex(device.pid)}`
}

function formatUsbHex(value: number) {
  return value.toString(16).padStart(4, '0').toUpperCase()
}

function clampMenuX(value: number) {
  return Math.max(12, Math.min(value, window.innerWidth - 320))
}

function clampMenuY(value: number) {
  return Math.max(12, Math.min(value, window.innerHeight - 320))
}
