import { useEffect } from 'react'
import { listenSerialBus } from './lib/tauri'
import { useAppStore } from './store/appStore'
import { Workbench } from './components/Workbench'

export default function App() {
  const bootstrap = useAppStore((state) => state.bootstrap)
  const ingestEvent = useAppStore((state) => state.ingestEvent)
  const session = useAppStore((state) => state.session)
  const status = useAppStore((state) => state.status)

  useEffect(() => {
    let cleanup: (() => void) | undefined

    void bootstrap()
    void listenSerialBus(ingestEvent).then((unlisten) => {
      cleanup = unlisten
    })

    return () => cleanup?.()
  }, [bootstrap, ingestEvent])

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
