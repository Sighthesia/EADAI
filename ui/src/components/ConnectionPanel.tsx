import { useAppStore } from '../store/appStore'

const FAKE_PROFILES = [
  { value: 'telemetry-lab', label: 'Telemetry Lab' },
  { value: 'noisy-monitor', label: 'Noisy Monitor' },
]

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
          <span>Source</span>
          <select
            value={config.sourceKind}
            onChange={(event) =>
              patchConfig({
                sourceKind: event.target.value as 'serial' | 'fake',
                port: event.target.value === 'serial' ? config.port : '',
              })
            }
          >
            <option value="serial">Serial Port</option>
            <option value="fake">Fake Stream</option>
          </select>
        </label>
        {config.sourceKind === 'fake' ? (
          <label>
            <span>Fake profile</span>
            <select
              value={config.fakeProfile ?? FAKE_PROFILES[0].value}
              onChange={(event) => patchConfig({ fakeProfile: event.target.value })}
            >
              {FAKE_PROFILES.map((profile) => (
                <option key={profile.value} value={profile.value}>
                  {profile.label}
                </option>
              ))}
            </select>
          </label>
        ) : (
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
        )}
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
        <button
          className="ghost-button"
          disabled={config.sourceKind === 'fake'}
          onClick={() => void refreshPorts()}
        >
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
