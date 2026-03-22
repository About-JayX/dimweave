import type { AgentInfo, DaemonStatus } from "../types";

interface AgentStatusProps {
  agents: Record<string, AgentInfo>;
  daemonStatus: DaemonStatus | null;
  connected: boolean;
  onLaunchCodexTui: () => void;
  onStopCodexTui: () => void;
}

const statusColors: Record<string, string> = {
  connected: "#22c55e",
  connecting: "#eab308",
  disconnected: "#6b7280",
  error: "#ef4444",
};

export function AgentStatusPanel({
  agents,
  daemonStatus,
  connected,
  onLaunchCodexTui,
  onStopCodexTui,
}: AgentStatusProps) {
  const codexTuiRunning = daemonStatus?.codexTuiRunning ?? false;
  const codexReady = daemonStatus?.codexBootstrapped ?? false;
  const claudeConnected = daemonStatus?.claudeConnected ?? false;

  return (
    <div style={styles.container}>
      {/* Daemon connection */}
      <div style={styles.header}>
        <h3 style={styles.title}>AgentBridge</h3>
        <span style={{ ...styles.dot, backgroundColor: connected ? "#22c55e" : "#ef4444" }} />
        <span style={styles.connLabel}>{connected ? "Online" : "Offline"}</span>
      </div>

      <div style={styles.agentList}>
        {/* Claude Code */}
        <div style={styles.agentCard}>
          <div style={styles.agentHeader}>
            <span style={{ ...styles.dot, backgroundColor: claudeConnected ? "#22c55e" : "#6b7280" }} />
            <span style={styles.agentName}>Claude Code</span>
            <span style={styles.statusLabel}>{claudeConnected ? "connected" : "disconnected"}</span>
          </div>
          {!claudeConnected && (
            <div style={styles.hint}>
              Register MCP in ~/.claude/mcp.json to connect
            </div>
          )}
        </div>

        {/* Codex */}
        <div style={styles.agentCard}>
          <div style={styles.agentHeader}>
            <span style={{
              ...styles.dot,
              backgroundColor: codexTuiRunning
                ? "#22c55e"
                : codexReady ? "#eab308" : "#6b7280",
            }} />
            <span style={styles.agentName}>Codex</span>
            <span style={styles.statusLabel}>
              {codexTuiRunning
                ? "connected"
                : codexReady ? "ready" : "starting..."}
            </span>
          </div>

          {daemonStatus?.threadId && (
            <div style={styles.detail}>Thread: {daemonStatus.threadId.slice(0, 16)}...</div>
          )}

          {/* Connect / Disconnect button */}
          <div style={styles.buttonRow}>
            {!codexTuiRunning ? (
              <button
                style={{
                  ...styles.actionBtn,
                  ...styles.connectBtn,
                  opacity: codexReady ? 1 : 0.4,
                }}
                disabled={!codexReady}
                onClick={onLaunchCodexTui}
              >
                Connect Codex
              </button>
            ) : (
              <button
                style={{ ...styles.actionBtn, ...styles.disconnectBtn }}
                onClick={onStopCodexTui}
              >
                Disconnect Codex
              </button>
            )}
          </div>

          {!codexReady && (
            <div style={styles.hint}>Codex app-server is starting...</div>
          )}
        </div>

        {/* Other agents placeholder */}
        {Object.entries(agents)
          .filter(([key]) => key !== "claude" && key !== "codex")
          .map(([key, agent]) => (
            <div key={key} style={styles.agentCard}>
              <div style={styles.agentHeader}>
                <span style={{ ...styles.dot, backgroundColor: statusColors[agent.status] }} />
                <span style={styles.agentName}>{agent.displayName}</span>
                <span style={styles.statusLabel}>{agent.status}</span>
              </div>
            </div>
          ))}
      </div>

      {/* Daemon info */}
      {daemonStatus && (
        <div style={styles.daemonInfo}>
          <div style={styles.sectionTitle}>Daemon</div>
          <div style={styles.detail}>PID: {daemonStatus.pid}</div>
          <div style={styles.detail}>Queued: {daemonStatus.queuedMessageCount}</div>
          <div style={styles.detail}>Proxy: {daemonStatus.proxyUrl}</div>
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: "16px",
    display: "flex",
    flexDirection: "column",
    gap: "12px",
    flex: 1,
  },
  header: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    paddingBottom: "12px",
    borderBottom: "1px solid #2d2d2d",
  },
  title: {
    margin: 0,
    fontSize: "14px",
    fontWeight: 600,
    color: "#e5e5e5",
    flex: 1,
  },
  dot: {
    width: "8px",
    height: "8px",
    borderRadius: "50%",
    display: "inline-block",
    flexShrink: 0,
  },
  connLabel: {
    fontSize: "12px",
    color: "#a3a3a3",
  },
  agentList: {
    display: "flex",
    flexDirection: "column",
    gap: "8px",
  },
  agentCard: {
    padding: "12px",
    backgroundColor: "#1e1e1e",
    borderRadius: "8px",
    border: "1px solid #333",
  },
  agentHeader: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
  },
  agentName: {
    fontSize: "13px",
    fontWeight: 500,
    color: "#e5e5e5",
    flex: 1,
  },
  statusLabel: {
    fontSize: "11px",
    color: "#a3a3a3",
    textTransform: "uppercase" as const,
  },
  detail: {
    fontSize: "11px",
    color: "#737373",
    marginTop: "4px",
    fontFamily: "monospace",
  },
  hint: {
    fontSize: "11px",
    color: "#525252",
    marginTop: "6px",
    lineHeight: 1.4,
  },
  buttonRow: {
    marginTop: "8px",
  },
  actionBtn: {
    width: "100%",
    padding: "6px 12px",
    fontSize: "12px",
    fontWeight: 500,
    border: "none",
    borderRadius: "6px",
    cursor: "pointer",
    transition: "opacity 0.15s",
  },
  connectBtn: {
    backgroundColor: "#22c55e",
    color: "#fff",
  },
  disconnectBtn: {
    backgroundColor: "#333",
    color: "#a3a3a3",
    border: "1px solid #404040",
  },
  sectionTitle: {
    fontSize: "11px",
    fontWeight: 600,
    color: "#737373",
    textTransform: "uppercase" as const,
    marginBottom: "4px",
  },
  daemonInfo: {
    padding: "10px",
    backgroundColor: "#1a1a1a",
    borderRadius: "6px",
    marginTop: "auto",
  },
};
