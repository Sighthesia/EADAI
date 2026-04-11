import { useEffect, useRef } from 'react'
import { listenSerialBus } from './lib/tauri'
import { useAppStore } from './store/appStore'
import type { SerialBusEvent } from './types'
import { Workbench } from './components/Workbench'

export default function App() {
  const bootstrap = useAppStore((state) => state.bootstrap)
  const connect = useAppStore((state) => state.connect)
  const ingestEvents = useAppStore((state) => state.ingestEvents)
  const session = useAppStore((state) => state.session)
  const status = useAppStore((state) => state.status)
  const pendingEventsRef = useRef<SerialBusEvent[]>([])
  const frameRef = useRef<number | null>(null)

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
      ingestEvents(queuedEvents)
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

      await bootstrap()

      const currentState = useAppStore.getState()
      if (!disposed && !currentState.session.isRunning && currentState.config.sourceKind === 'fake') {
        await connect()
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
        ingestEvents(queuedEvents)
      }
    }
  }, [bootstrap, connect, ingestEvents])

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div>
          <strong>EADAI Workbench</strong>
          <small>Serial analysis workspace with dockable panels</small>
        </div>
        <div className={`status-pill tone-${status.tone}`}>
          <span className="status-dot" />
          {session.port ? `${session.port} · ${session.connectionState ?? 'idle'}` : status.message}
        </div>
      </header>
      <section className="status-strip">
        <span>{status.message}</span>
      </section>
      <section className="workbench-shell">
        <Workbench />
      </section>
    </main>
  )
}
