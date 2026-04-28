import { useAppStore } from '../store/appStore'
import { RuntimeConsoleSection } from './RuntimeConsoleSection'

export function RuntimePanel() {
  const protocol = useAppStore((state) => state.protocol)
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const sentConsoleEntries = useAppStore((state) => state.sentConsoleEntries)
  const consoleDisplayMode = useAppStore((state) => state.consoleDisplayMode)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const runtimeCatalog = useAppStore((state) => state.runtimeCatalog)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const setConsoleDisplayMode = useAppStore((state) => state.setConsoleDisplayMode)
  const send = useAppStore((state) => state.send)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)

  return (
    <section className="panel runtime-panel">
      <RuntimeConsoleSection
        runtimeCatalog={runtimeCatalog}
        protocolPhase={protocol.phase}
        commandInput={commandInput}
        consoleDisplayMode={consoleDisplayMode}
        consoleEntries={consoleEntries}
        sentConsoleEntries={sentConsoleEntries}
        appendNewline={appendNewline}
        onCommandInputChange={setCommandInput}
        onAppendNewlineChange={setAppendNewline}
        onConsoleDisplayModeChange={setConsoleDisplayMode}
        onSend={() => void send()}
        onSendCommand={(command, payload) => void sendBmi088Command(command, payload)}
      />
    </section>
  )
}
