import { useMemo } from 'react'
import { useAppStore } from '../store/appStore'
import type { UiRuntimeDeviceSnapshot } from '../types'
import { RuntimeCommandCenter } from './RuntimeCommandCenter'
import { RuntimeCatalogSection } from './RuntimeCatalogSection'
import { RuntimeConsoleSection } from './RuntimeConsoleSection'
import { RuntimeHookSection } from './RuntimeHookSection'
import { RuntimeOverviewSection } from './RuntimeOverviewSection'
import { RuntimeProtocolSection } from './RuntimeProtocolSection'
import { buildHookStatus, collectRecentTriggers } from './runtimeUtils'

export function RuntimePanel() {
  const protocol = useAppStore((state) => state.protocol)
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const consoleDisplayMode = useAppStore((state) => state.consoleDisplayMode)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const protocolScript = useAppStore((state) => state.protocolScript)
  const protocolHookExamples = useAppStore((state) => state.protocolHookExamples)
  const runtimeCatalog = useAppStore((state) => state.runtimeCatalog)
  const runtimeDevice = useAppStore((state) => state.runtimeDevice)
  const variables = useAppStore((state) => state.variables)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const setConsoleDisplayMode = useAppStore((state) => state.setConsoleDisplayMode)
  const send = useAppStore((state) => state.send)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)

  const recentTraffic = useMemo(() => consoleEntries.slice(-5).reverse(), [consoleEntries])
  const recentTimeline = useMemo(() => protocol.timeline.slice(-5).reverse(), [protocol.timeline])
  const recentTriggers = useMemo(() => collectRecentTriggers(variables), [variables])
  const hookStatus = useMemo(() => buildHookStatus(protocolScript, protocolHookExamples.length, recentTriggers.length), [protocolHookExamples.length, protocolScript, recentTriggers.length])

  return (
    <section className="panel panel-scroll runtime-panel">
      <div className="runtime-header">
        <div>
          <span className="mcp-label">Runtime surface</span>
          <h2>Runtime state at a glance</h2>
        </div>
        <div className="runtime-header-meta">
          <span className={`metric-chip ${protocol.active ? 'tone-success' : ''}`}>{protocol.active ? 'Active' : 'Idle'}</span>
          <span className="metric-chip">{protocol.parserName}</span>
        </div>
      </div>

      <RuntimeOverviewSection
        runtimeDevice={runtimeDevice}
        protocolActive={protocol.active}
        parserName={protocol.parserName}
        protocolPhase={protocol.phase}
        transportLabel={protocol.transportLabel}
        baudRate={protocol.baudRate}
        lastHandshakeAtMs={protocol.lastHandshakeAtMs ?? null}
        lastPacketKind={protocol.lastPacketKind ?? null}
        consoleEntries={consoleEntries}
        recentTraffic={recentTraffic}
        hookStatus={hookStatus}
        protocolHookExamples={protocolHookExamples}
      />

      <div className="runtime-inspector-grid">
        <RuntimeCommandCenter
          runtimeDevice={runtimeDevice}
          protocolPhase={protocol.phase}
          runtimeCatalog={runtimeCatalog}
          onSendCommand={(command) => void sendBmi088Command(command)}
        />
        <RuntimeConsoleSection runtimeCommands={runtimeCatalog.commands} commandInput={commandInput} consoleDisplayMode={consoleDisplayMode} consoleEntries={consoleEntries} appendNewline={appendNewline} onCommandInputChange={setCommandInput} onAppendNewlineChange={setAppendNewline} onDisplayModeChange={setConsoleDisplayMode} onSend={() => void send()} onSendCommand={(command) => void sendBmi088Command(command)} />
        <RuntimeProtocolSection protocol={protocol} recentTimeline={recentTimeline} runtimeCommands={runtimeCatalog.commands} onSendCommand={(command) => void sendBmi088Command(command)} />
        <RuntimeCatalogSection runtimeCatalog={runtimeCatalog} />
        <RuntimeHookSection hookStatus={hookStatus} protocolScript={protocolScript} protocolHookExamples={protocolHookExamples} recentTriggers={recentTriggers} />
      </div>
    </section>
  )
}
