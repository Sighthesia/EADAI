import { useEffect, useRef } from 'react'
import { listenSerialBus, listenSerialDevicesChanged } from './lib/tauri'
import { useAppStore } from './store/appStore'
import type { SerialBusEvent } from './types'
import { Workbench } from './components/Workbench'
import { shouldPollMcpStatus } from './store/appStore'

const SERIAL_PORT_POLL_INTERVAL_MS = 1500
const SHOULD_POLL_SERIAL_PORTS = !navigator.userAgent.toLowerCase().includes('linux')
const LEFT_DOCK_BUTTON_SELECTOR = '.flexlayout__border_left .flexlayout__mini_scrollbar_container > div'
const LEFT_DOCK_BUTTON_WIDTH_VAR = '--left-dock-settings-width'

export default function App() {
  const mcp = useAppStore((state) => state.mcp)
  const refreshMcpStatus = useAppStore((state) => state.refreshMcpStatus)
  const refreshPortsSilently = useAppStore((state) => state.refreshPortsSilently)
  const refreshLogicAnalyzerStatus = useAppStore((state) => state.refreshLogicAnalyzerStatus)
  const session = useAppStore((state) => state.session)
  const logicAnalyzerSessionState = useAppStore((state) => state.logicAnalyzer.sessionState)
  const status = useAppStore((state) => state.status)
  const pendingEventsRef = useRef<SerialBusEvent[]>([])
  const frameRef = useRef<number | null>(null)
  const connectionSummary = session.port ? `${session.port} · ${session.connectionState ?? 'idle'}` : status.message
  const runtimeSummary = `MCP ${mcp.isRunning ? 'ready' : 'starting'} · Logic ${logicAnalyzerSessionState}`

  useEffect(() => {
    const root = document.documentElement
    const syncLeftDockButtonWidth = () => {
      const leftDockButton = document.querySelector<HTMLElement>(LEFT_DOCK_BUTTON_SELECTOR)
      const width = leftDockButton?.getBoundingClientRect().width ?? 44

      root.style.setProperty(LEFT_DOCK_BUTTON_WIDTH_VAR, `${Math.ceil(width)}px`)
    }

    syncLeftDockButtonWidth()

    const resizeObserver = new ResizeObserver(() => {
      syncLeftDockButtonWidth()
    })
    const mutationObserver = new MutationObserver(() => {
      syncLeftDockButtonWidth()
      const leftDockButton = document.querySelector<HTMLElement>(LEFT_DOCK_BUTTON_SELECTOR)
      if (leftDockButton) {
        resizeObserver.observe(leftDockButton)
      }
    })

    mutationObserver.observe(document.body, { childList: true, subtree: true, attributes: true })

    const leftDockButton = document.querySelector<HTMLElement>(LEFT_DOCK_BUTTON_SELECTOR)
    if (leftDockButton) {
      resizeObserver.observe(leftDockButton)
    }

    return () => {
      resizeObserver.disconnect()
      mutationObserver.disconnect()
      root.style.removeProperty(LEFT_DOCK_BUTTON_WIDTH_VAR)
    }
  }, [])

  useEffect(() => {
    let disposed = false
    let cleanup: (() => void) | undefined

    const flushPendingEvents = () => {
      frameRef.current = null
      if (pendingEventsRef.current.length === 0) {
        return
      }

      const queuedEvents = pendingEventsRef.current
      pendingEventsRef.current = []
      useAppStore.getState().ingestEvents(queuedEvents)
    }

    const queueEvent = (event: SerialBusEvent) => {
      pendingEventsRef.current.push(event)
      if (frameRef.current === null) {
        frameRef.current = window.requestAnimationFrame(flushPendingEvents)
      }
    }

    const initialize = async () => {
      const unlisten = await listenSerialBus(queueEvent)
      if (disposed) {
        void unlisten()
        return
      }
      cleanup = unlisten

      await useAppStore.getState().bootstrap()

      const currentState = useAppStore.getState()
      if (!disposed && !currentState.session.isRunning && currentState.config.sourceKind === 'fake') {
        await useAppStore.getState().connect()
      }
    }

    void initialize()

    return () => {
      disposed = true
      cleanup?.()

      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current)
        frameRef.current = null
      }

      if (pendingEventsRef.current.length > 0) {
        const queuedEvents = pendingEventsRef.current
        pendingEventsRef.current = []
        useAppStore.getState().ingestEvents(queuedEvents)
      }
    }
  }, [])

  useEffect(() => {
    let disposed = false
    let cleanup: (() => void) | undefined

    const initialize = async () => {
      const unlisten = await listenSerialDevicesChanged(() => {
        void refreshPortsSilently()
      })

      if (disposed) {
        void unlisten()
        return
      }

      cleanup = unlisten
    }

    void initialize()

    return () => {
      disposed = true
      cleanup?.()
    }
  }, [refreshPortsSilently])

  useEffect(() => {
    if (!SHOULD_POLL_SERIAL_PORTS) {
      return
    }

    const interval = window.setInterval(() => {
      void refreshPortsSilently()
    }, SERIAL_PORT_POLL_INTERVAL_MS)

    return () => {
      window.clearInterval(interval)
    }
  }, [refreshPortsSilently])

  useEffect(() => {
    if (logicAnalyzerSessionState !== 'capturing') {
      return
    }

    const interval = window.setInterval(() => {
      void refreshLogicAnalyzerStatus()
    }, 1500)

    return () => {
      window.clearInterval(interval)
    }
  }, [logicAnalyzerSessionState, refreshLogicAnalyzerStatus])

  useEffect(() => {
    if (!shouldPollMcpStatus(mcp)) {
      return
    }

    const interval = window.setInterval(() => {
      void refreshMcpStatus()
    }, 1000)

    return () => {
      window.clearInterval(interval)
    }
  }, [mcp, refreshMcpStatus])

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div>
          <strong>EADAI Workbench</strong>
        </div>
      </header>
      <section className="workbench-shell">
        <Workbench />
      </section>
      <section className="status-strip">
        <div className="status-strip-gutter" aria-hidden="true" />
        <div className="status-strip-body">
          <div className={`status-pill tone-${status.tone}`}>
            <span className="status-dot" />
            {connectionSummary}
          </div>
          <span className="status-strip-runtime">{runtimeSummary}</span>
          <span className="status-strip-message">{status.message}</span>
        </div>
      </section>
    </main>
  )
}
