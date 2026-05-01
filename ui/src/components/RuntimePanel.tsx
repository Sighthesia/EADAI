import { useMemo } from 'react'
import { useAppStore } from '../store/appStore'
import { RuntimeCatalogSection } from './RuntimeCatalogSection'
import { RuntimeCommandCenter } from './RuntimeCommandCenter'
import { RuntimeConsoleSection } from './RuntimeConsoleSection'
import { RuntimeHookSection } from './RuntimeHookSection'
import { RuntimeOverviewSection } from './RuntimeOverviewSection'
import { buildRuntimeActivityStatus, collectRecentTriggers } from './runtimeUtils'

export function RuntimePanel() {
  const protocolPhase = useAppStore((state) => state.protocol.phase)
  const protocolActive = useAppStore((state) => state.protocol.active)
  const parserName = useAppStore((state) => state.protocol.parserName)
  const transportLabel = useAppStore((state) => state.protocol.transportLabel)
  const baudRate = useAppStore((state) => state.protocol.baudRate)
  const lastHandshakeAtMs = useAppStore((state) => state.protocol.lastHandshakeAtMs)
  const lastPacketKind = useAppStore((state) => state.protocol.lastPacketKind)
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const sentConsoleEntries = useAppStore((state) => state.sentConsoleEntries)
  const consoleDisplayMode = useAppStore((state) => state.consoleDisplayMode)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const runtimeCatalog = useAppStore((state) => state.runtimeCatalog)
  const runtimeDevice = useAppStore((state) => state.runtimeDevice)
  const variables = useAppStore((state) => state.variables)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const setConsoleDisplayMode = useAppStore((state) => state.setConsoleDisplayMode)
  const send = useAppStore((state) => state.send)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)

  const recentTriggers = useMemo(() => collectRecentTriggers(variables), [variables])
  const hookStatus = useMemo(() => buildRuntimeActivityStatus(recentTriggers.length), [recentTriggers.length])
  const recentTraffic = useMemo(() => [...consoleEntries].slice(-5).reverse(), [consoleEntries])

  return (
    <section className="panel runtime-panel">
      <RuntimeOverviewSection
        runtimeDevice={runtimeDevice}
        protocolActive={protocolActive}
        parserName={parserName}
        protocolPhase={protocolPhase}
        transportLabel={transportLabel}
        baudRate={baudRate}
        lastHandshakeAtMs={lastHandshakeAtMs}
        lastPacketKind={lastPacketKind}
        consoleEntries={consoleEntries}
        recentTraffic={recentTraffic}
        hookStatus={hookStatus}
      />

      <div className="runtime-session-layout">
        <div className="runtime-session-primary">
          <RuntimeConsoleSection
            protocolPhase={protocolPhase}
            commandInput={commandInput}
            consoleDisplayMode={consoleDisplayMode}
            consoleEntries={consoleEntries}
            sentConsoleEntries={sentConsoleEntries}
            appendNewline={appendNewline}
            onCommandInputChange={setCommandInput}
            onAppendNewlineChange={setAppendNewline}
            onConsoleDisplayModeChange={setConsoleDisplayMode}
            onSend={() => void send()}
          />
        </div>

        <div className="runtime-session-sidebar">
          <RuntimeCommandCenter runtimeCatalog={runtimeCatalog} protocolPhase={protocolPhase} onSendCommand={(command, payload) => void sendBmi088Command(command, payload)} />
          <RuntimeHookSection hookStatus={hookStatus} recentTriggers={recentTriggers} />
          <RuntimeCatalogSection runtimeCatalog={runtimeCatalog} />
        </div>
      </div>
    </section>
  )
}
