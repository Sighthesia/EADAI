import { useAppStore } from '../store/appStore'

export function ScriptHookPanel() {
  const protocolScript = useAppStore((state) => state.protocolScript)
  const protocolHookExamples = useAppStore((state) => state.protocolHookExamples)

  return (
    <section className="panel panel-scroll script-hook-panel">
      <div className="variables-header">
        <span>Script / hook</span>
        <div className="variables-header-actions">
          <span>{protocolHookExamples.length} examples</span>
        </div>
      </div>

      <section className="script-hook-card">
        <div className="protocol-schema-header">
          <strong>BMI088 hooks</strong>
          <small>Use structured schema and sample callbacks</small>
        </div>
        <pre>{protocolScript}</pre>
      </section>

      <div className="script-hook-grid">
        {protocolHookExamples.map((example) => (
          <article key={example.name} className="protocol-card">
            <span className="mcp-label">{example.name}</span>
            <pre>{example.snippet}</pre>
          </article>
        ))}
      </div>
    </section>
  )
}
