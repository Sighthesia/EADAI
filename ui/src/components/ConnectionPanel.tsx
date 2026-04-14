import { SourceChoiceGroup } from './SourceChoiceGroup'
import { useAppStore } from '../store/appStore'

const FAKE_PROFILES = [
  { value: 'telemetry-lab', label: 'Telemetry Lab' },
  { value: 'noisy-monitor', label: 'Noisy Monitor' },
  { value: 'imu-lab', label: 'IMU Lab' },
]

const SOURCE_OPTIONS = [
  { value: 'serial' as const, label: 'Serial Port', description: 'Use a live serial device connected to the machine.' },
  { value: 'fake' as const, label: 'Fake Stream', description: 'Use built-in synthetic telemetry for UI and analysis.' },
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
      <SourceChoiceGroup
        ariaLabel="Connection source"
        className="connection-source-switch"
        value={config.sourceKind}
        options={SOURCE_OPTIONS}
        onChange={(value) =>
          patchConfig({
            sourceKind: value,
            port: value === 'serial' ? config.port : '',
          })
        }
      />
      <div className="panel-grid">
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
