import { useAppStore } from '../store/appStore'
import { RuntimeConsoleSection } from './RuntimeConsoleSection'

export function RuntimePanel() {
  const protocolPhase = useAppStore((state) => state.protocol.phase)
  const consoleEntries = useAppStore((state) => state.consoleEntries)
  const sentConsoleEntries = useAppStore((state) => state.sentConsoleEntries)
  const consoleDisplayMode = useAppStore((state) => state.consoleDisplayMode)
  const commandInput = useAppStore((state) => state.commandInput)
  const appendNewline = useAppStore((state) => state.appendNewline)
  const runtimeCommands = useAppStore((state) => state.runtimeCatalog.commands)
  const setCommandInput = useAppStore((state) => state.setCommandInput)
  const setAppendNewline = useAppStore((state) => state.setAppendNewline)
  const setConsoleDisplayMode = useAppStore((state) => state.setConsoleDisplayMode)
  const send = useAppStore((state) => state.send)
  const sendBmi088Command = useAppStore((state) => state.sendBmi088Command)

  return (
    <section className="panel runtime-panel">
      <RuntimeConsoleSection
        runtimeCommands={runtimeCommands}
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
        onSendCommand={(command, payload) => void sendBmi088Command(command, payload)}
      />
    </section>
  )
}
