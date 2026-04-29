import { useAppStore } from '../store/appStore'
import type { McpToolUsageSnapshot, SessionSnapshot } from '../types'
import { formatTime } from './runtimeUtils'

export function McpPanel() {
  const mcp = useAppStore((state) => state.mcp)
  const mcpToolUsage = useAppStore((state) => state.mcpToolUsage)
  const session = useAppStore((state) => state.session)
  const refreshMcpStatus = useAppStore((state) => state.refreshMcpStatus)
  const refreshMcpToolUsage = useAppStore((state) => state.refreshMcpToolUsage)
  const analysisJson = useAppStore((state) =>
    JSON.stringify(
      Object.values(state.variables)
        .filter((variable) => variable.analysis)
        .map((variable) => variable.analysis),
      null,
      2,
    ),
  )

  const copyAnalysisJson = async () => {
    await navigator.clipboard.writeText(analysisJson)
  }

  const toolCards = mcpToolUsage.length > 0 ? mcpToolUsage : defaultToolCards()

  return (
    <section className="panel panel-scroll mcp-panel">
      <div className="variables-header">
        <span>Shared MCP Status</span>
        <div className="panel-actions">
          <button className="ghost-button" onClick={() => void copyAnalysisJson()}>
            Copy Analysis JSON
          </button>
          <button className="ghost-button" onClick={() => void refreshMcpStatus()}>
            Refresh MCP
          </button>
          <button className="ghost-button" onClick={() => void refreshMcpToolUsage()}>
            Refresh Tools
          </button>
        </div>
      </div>

      <details className="runtime-disclosure" open>
        <summary>
          <strong>MCP server summary</strong>
          <small>{mcp.isRunning ? 'Running' : 'Starting'}</small>
        </summary>
        <div className="runtime-disclosure-body">
          <div className="mcp-card-grid">
            <article className="mcp-card">
              <span className="mcp-label">Server</span>
              <strong>{mcpStatusLabel(mcp)}</strong>
              <small>{mcp.transport}</small>
            </article>
            <article className="mcp-card">
              <span className="mcp-label">Endpoint</span>
              <strong className="mcp-endpoint">{mcp.endpointUrl ?? 'starting...'}</strong>
              <small>{mcp.isRunning ? 'Desktop MCP endpoint for Claude/Codex.' : 'Waiting for embedded MCP bind.'}</small>
            </article>
            <article className="mcp-card">
              <span className="mcp-label">Runtime Source</span>
              <strong>{runtimeSourceLabel(session)}</strong>
              <small>{runtimeSyncLabel(session)}</small>
            </article>
          </div>

          {mcp.lastError ? (
            <div className="mcp-banner mcp-banner-warning">
              <strong>Bind warning</strong>
              <span>{mcp.lastError}</span>
            </div>
          ) : null}

          <dl className="detail-list">
            <div>
              <dt>Session status</dt>
              <dd>{session.connectionState ?? 'stopped'}</dd>
            </div>
            <div>
              <dt>Transport</dt>
              <dd>{session.transport ?? '-'}</dd>
            </div>
            <div>
              <dt>Port</dt>
              <dd>{session.port ?? '-'}</dd>
            </div>
            <div>
              <dt>Baud</dt>
              <dd>{session.baudRate ?? '-'}</dd>
            </div>
            <div>
              <dt>Sharing</dt>
              <dd>{session.isRunning ? 'UI and MCP read the same live session.' : 'MCP is ready and will follow the next session.'}</dd>
            </div>
            <div>
              <dt>Mode</dt>
              <dd>Desktop embedded MCP</dd>
            </div>
          </dl>
        </div>
      </details>

      <details className="runtime-disclosure" open>
        <summary>
          <strong>MCP tools</strong>
          <small>{toolCards.length} tools</small>
        </summary>
        <div className="runtime-disclosure-body">
          <div className="mcp-tool-grid">
            {toolCards.map((tool) => (
              <article key={tool.name} className="variable-card mcp-tool-card">
                <div className="variable-main">
                  <span className="mcp-label">Tool</span>
                  <strong>{tool.name}</strong>
                  <small>{tool.lastCalledAtMs ? `Last called ${formatTime(tool.lastCalledAtMs)}` : 'Never called in this session'}</small>
                </div>
                <div className="variable-metric">
                  <span className="metric-chip">{tool.lastCalledAtMs ? 'active' : 'idle'}</span>
                </div>
              </article>
            ))}
          </div>
        </div>
      </details>
    </section>
  )
}

const defaultToolCards = (): McpToolUsageSnapshot[] => [
  { name: 'get_channel_analysis', lastCalledAtMs: null },
  { name: 'get_recent_events', lastCalledAtMs: null },
  { name: 'get_channel_statistics', lastCalledAtMs: null },
  { name: 'query_historical_analysis', lastCalledAtMs: null },
]

const mcpStatusLabel = (mcp: { isRunning: boolean; lastError?: string | null }) => {
  if (mcp.isRunning) {
    return 'running'
  }

  if (mcp.lastError) {
    return 'degraded'
  }

  return 'starting'
}

const runtimeSourceLabel = (session: SessionSnapshot) => {
  if (!session.isRunning) {
    return 'idle'
  }

  if (session.transport === 'fake') {
    return session.port ?? 'fake stream'
  }

  return session.port ?? 'serial session'
}

const runtimeSyncLabel = (session: SessionSnapshot) => {
  if (!session.isRunning) {
    return 'Start serial or fake mode and MCP follows automatically.'
  }

  return session.transport === 'fake'
    ? 'Current fake stream is shared with MCP clients.'
    : 'Current serial session is shared with MCP clients.'
}
