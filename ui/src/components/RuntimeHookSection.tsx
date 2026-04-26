import type { UiTriggerPayload } from '../types'
import { RuntimeSectionHeader, RuntimeTriggerRow } from './runtimeUtils'

export function RuntimeHookSection({
  hookStatus,
  recentTriggers,
}: {
  hookStatus: { label: string; detail: string }
  recentTriggers: UiTriggerPayload[]
}) {
  return (
    <section className="runtime-section runtime-hook-section">
      <RuntimeSectionHeader title="Runtime activity" description="Live trigger activity and the current execution state" />
      <div className="runtime-summary-grid runtime-hook-summary-grid">
        <article className="runtime-card">
          <span className="mcp-label">Status</span>
          <strong>{hookStatus.label}</strong>
          <small>{hookStatus.detail}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Triggers</span>
          <strong>{recentTriggers.length}</strong>
          <small>Recent variable trigger events</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Focus</span>
          <strong>Inspection only</strong>
          <small>Definition content lives in the Scripts surface now.</small>
        </article>
      </div>

      <div className="runtime-hook-grid runtime-hook-stack">
        <article className="runtime-hook-card">
          <span className="mcp-label">Recent triggers</span>
          <div className="runtime-trigger-list">
            {recentTriggers.length > 0 ? recentTriggers.map((trigger) => <RuntimeTriggerRow key={`${trigger.ruleId}-${trigger.channelId}-${trigger.firedAtMs}`} trigger={trigger} />) : <div className="protocol-empty">No hook triggers yet.</div>}
          </div>
        </article>
      </div>
    </section>
  )
}
