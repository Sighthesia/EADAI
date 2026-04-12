import { useAppStore } from '../store/appStore'
import type { SessionSnapshot } from '../types'

export function McpPanel() {
  const mcp = useAppStore((state) => state.mcp)
  const session = useAppStore((state) => state.session)
  const refreshMcpStatus = useAppStore((state) => state.refreshMcpStatus)

  return (
    <section className="panel panel-scroll mcp-panel">
      <div className="variables-header">
        <span>Shared MCP Status</span>
        <button className="ghost-button" onClick={() => void refreshMcpStatus()}>
          Refresh MCP
        </button>
      </div>

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
    </section>
  )
}

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
