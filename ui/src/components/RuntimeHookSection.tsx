import type { UiTriggerPayload } from '../types'
import { RuntimeSectionHeader, RuntimeTriggerRow } from './runtimeUtils'

export function RuntimeHookSection({
  hookStatus,
  protocolScript,
  protocolHookExamples,
  recentTriggers,
}: {
  hookStatus: { label: string; detail: string }
  protocolScript: string
  protocolHookExamples: { name: string; snippet: string }[]
  recentTriggers: UiTriggerPayload[]
}) {
  return (
    <section className="runtime-section runtime-hook-section">
      <RuntimeSectionHeader title="Hook inspector" description="Shared script, reference snippets, and recent trigger activity" />
      <div className="runtime-summary-grid runtime-hook-summary-grid">
        <article className="runtime-card">
          <span className="mcp-label">Status</span>
          <strong>{hookStatus.label}</strong>
          <small>{hookStatus.detail}</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Examples</span>
          <strong>{protocolHookExamples.length}</strong>
          <small>Available hook snippets</small>
        </article>
        <article className="runtime-card">
          <span className="mcp-label">Triggers</span>
          <strong>{recentTriggers.length}</strong>
          <small>Recent variable trigger events</small>
        </article>
      </div>

      <div className="runtime-hook-grid runtime-hook-stack">
        <article className="runtime-hook-card">
          <span className="mcp-label">Script</span>
          <pre>{protocolScript}</pre>
        </article>
        <article className="runtime-hook-card">
          <span className="mcp-label">Examples</span>
          <div className="runtime-example-list">
            {protocolHookExamples.map((example) => (
              <div key={example.name} className="runtime-example-item">
                <strong>{example.name}</strong>
                <pre>{example.snippet}</pre>
              </div>
            ))}
          </div>
        </article>
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
