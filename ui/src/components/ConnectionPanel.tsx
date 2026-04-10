import { useAppStore } from '../store/appStore'

export function ConnectionPanel() {
  const ports = useAppStore((state) => state.ports)
  const config = useAppStore((state) => state.config)
  const session = useAppStore((state) => state.session)
  const refreshPorts = useAppStore((state) => state.refreshPorts)
  const connect = useAppStore((state) => state.connect)
  const disconnect = useAppStore((state) => state.disconnect)
  const patchConfig = useAppStore((state) => state.patchConfig)

  return (
    <section className="panel panel-form">
      <div className="panel-grid">
        <label>
          <span>Port</span>
          <select value={config.port} onChange={(event) => patchConfig({ port: event.target.value })}>
            <option value="">Select serial port</option>
            {ports.map((port) => (
              <option key={port} value={port}>
                {port}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>Baud</span>
          <input
            type="number"
            value={config.baudRate}
            onChange={(event) => patchConfig({ baudRate: Number(event.target.value) || 115200 })}
          />
        </label>
        <label>
          <span>Retry ms</span>
          <input
            type="number"
            value={config.retryMs}
            onChange={(event) => patchConfig({ retryMs: Number(event.target.value) || 1000 })}
          />
        </label>
        <label>
          <span>Read timeout ms</span>
          <input
            type="number"
            value={config.readTimeoutMs}
            onChange={(event) => patchConfig({ readTimeoutMs: Number(event.target.value) || 50 })}
          />
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
          <button className="primary-button" onClick={() => void connect()}>
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
          <dt>Port</dt>
          <dd>{session.port ?? '-'}</dd>
        </div>
        <div>
          <dt>Baud</dt>
          <dd>{session.baudRate ?? '-'}</dd>
        </div>
      </dl>
    </section>
  )
}
