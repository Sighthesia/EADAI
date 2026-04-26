import type { ConsoleEntry, UiRuntimeDeviceSnapshot } from '../types'
import { RuntimeFlowStep, RuntimeSectionHeader, formatTime, summarizeLatestTraffic, summarizeTraffic, type HookStatus } from './runtimeUtils'

export function RuntimeOverviewSection({
  runtimeDevice,
  protocolActive,
  parserName,
  protocolPhase,
  transportLabel,
  baudRate,
  lastHandshakeAtMs,
  lastPacketKind,
  consoleEntries,
  recentTraffic,
  hookStatus,
}: {
  runtimeDevice: UiRuntimeDeviceSnapshot
  protocolActive: boolean
  parserName: string
  protocolPhase: string
  transportLabel: string
  baudRate: number
  lastHandshakeAtMs: number | null | undefined
  lastPacketKind: string | null | undefined
  consoleEntries: ConsoleEntry[]
  recentTraffic: ConsoleEntry[]
  hookStatus: HookStatus
}) {
  return (
    <section className="runtime-overview" aria-label="Runtime summary and flow">
      <RuntimeSectionHeader title="Runtime state at a glance" description="Device, protocol, traffic, and hook state in one place" />
      <div className="runtime-summary-grid runtime-dashboard-summary-grid">
        <article className="runtime-card runtime-device-card runtime-device-summary-card">
          <span className="mcp-label">Device</span>
          <strong>{runtimeDevice.label}</strong>
          <small>{runtimeDevice.detail}</small>
          <small>{runtimeDevice.transportLabel} · {runtimeDevice.status}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Protocol</span>
          <strong>{parserName}</strong>
          <small>{transportLabel}</small>
          <small>{protocolActive ? 'Connected parser' : 'Waiting for traffic'}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Handshake</span>
          <strong>{protocolPhase}</strong>
          <small>Last sync: {formatTime(lastHandshakeAtMs ?? null)}</small>
          <small>Last packet: {lastPacketKind ?? '-'}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Traffic</span>
          <strong>{consoleEntries.length} lines</strong>
          <small>{summarizeLatestTraffic(recentTraffic[0] ?? null)}</small>
          <small>Raw traffic feeds the parsed protocol view</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Runtime activity</span>
          <strong>{hookStatus.label}</strong>
          <small>{hookStatus.detail}</small>
          <small>Definition details live in the Scripts surface.</small>
        </article>
      </div>

      <div className="runtime-flow-strip" aria-label="Runtime flow summary">
        <RuntimeFlowStep label="Serial traffic" value={summarizeTraffic(consoleEntries)} detail={summarizeLatestTraffic(recentTraffic[0] ?? null)} />
        <RuntimeFlowStep label="Protocol state" value={protocolPhase} detail={`${transportLabel} · ${baudRate} baud`} />
        <RuntimeFlowStep label="Hook runtime" value={hookStatus.label} detail={hookStatus.detail} />
      </div>
    </section>
  )
}
