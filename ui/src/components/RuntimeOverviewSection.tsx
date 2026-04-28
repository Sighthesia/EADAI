import type { ConsoleEntry, UiRuntimeDeviceSnapshot } from '../types'
import { RuntimeSectionHeader, formatTime, summarizeLatestTraffic, type HookStatus } from './runtimeUtils'

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
      <RuntimeSectionHeader title="Current state" />
      <div className="runtime-compact-strip">
        <article className="runtime-card runtime-device-card runtime-device-summary-card">
          <span className="mcp-label">Device</span>
          <strong>{runtimeDevice.label}</strong>
          <small>{runtimeDevice.detail}</small>
          <small>{runtimeDevice.transportLabel} · {runtimeDevice.status}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Protocol</span>
          <strong>{parserName}</strong>
          <small>{protocolPhase} · {transportLabel}</small>
          <small>{protocolActive ? `Last sync ${formatTime(lastHandshakeAtMs ?? null)}` : 'Waiting for traffic'}</small>
        </article>
        <article className="runtime-card runtime-compact-card">
          <span className="mcp-label">Traffic</span>
          <strong>{consoleEntries.length} lines</strong>
          <small>{summarizeLatestTraffic(recentTraffic[0] ?? null)}</small>
          <small>{baudRate} baud</small>
        </article>
        <article className="runtime-card runtime-compact-card">
            <span className="mcp-label">Terminal activity</span>
          <strong>{hookStatus.label}</strong>
          <small>{hookStatus.detail}</small>
          <small>Last packet: {lastPacketKind ?? '-'}</small>
        </article>
      </div>
    </section>
  )
}
