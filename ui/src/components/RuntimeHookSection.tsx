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
      <RuntimeSectionHeader title="Trigger diagnostics" />
      <div className="runtime-summary-grid runtime-hook-summary-grid">
        <article className="runtime-card">
          <span className="mcp-label">Status</span>
          <strong>{hookStatus.label}</strong>
          <small>{hookStatus.detail}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Triggers</span>
          <strong>{recentTriggers.length}</strong>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Focus</span>
          <strong>Inspection only</strong>
        </article>
      </div>

      <details className="runtime-disclosure">
        <summary>
          <strong>Recent triggers</strong>
          <small>{recentTriggers.length > 0 ? `${recentTriggers.length} recent events` : 'No trigger activity yet'}</small>
        </summary>
        <div className="runtime-disclosure-body runtime-hook-grid runtime-hook-stack">
          <article className="runtime-hook-card">
            <div className="runtime-trigger-list">
              {recentTriggers.length > 0 ? recentTriggers.map((trigger) => <RuntimeTriggerRow key={`${trigger.ruleId}-${trigger.channelId}-${trigger.firedAtMs}`} trigger={trigger} />) : <div className="protocol-empty">No hook triggers yet.</div>}
            </div>
          </article>
        </div>
      </details>
    </section>
  )
}
