import { useBridgeWebSocket } from "./hooks/useWebSocket";
import { AgentStatusPanel } from "./components/AgentStatus";
import { MessagePanel } from "./components/MessagePanel";
import { ReplyInput } from "./components/ReplyInput";

export default function App() {
  const {
    connected, messages, agents, daemonStatus,
    sendToCodex, clearMessages, launchCodexTui, stopCodexTui,
  } = useBridgeWebSocket();

  return (
    <div style={styles.app}>
      <div style={styles.sidebar}>
        <div style={styles.logo}>
          <h2 style={styles.logoText}>AgentBridge</h2>
          <span style={styles.version}>v0.1.0</span>
        </div>
        <AgentStatusPanel
          agents={agents}
          daemonStatus={daemonStatus}
          connected={connected}
          onLaunchCodexTui={launchCodexTui}
          onStopCodexTui={stopCodexTui}
        />
      </div>

      <div style={styles.main}>
        <MessagePanel messages={messages} onClear={clearMessages} />
        <ReplyInput onSend={sendToCodex} disabled={!connected} />
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  app: {
    display: "flex",
    height: "100vh",
    backgroundColor: "#0a0a0a",
    color: "#e5e5e5",
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
  },
  sidebar: {
    width: "280px",
    borderRight: "1px solid #2d2d2d",
    display: "flex",
    flexDirection: "column",
    flexShrink: 0,
  },
  logo: {
    display: "flex",
    alignItems: "baseline",
    gap: "8px",
    padding: "16px",
    borderBottom: "1px solid #2d2d2d",
  },
  logoText: {
    margin: 0,
    fontSize: "16px",
    fontWeight: 700,
    color: "#f5f5f5",
  },
  version: {
    fontSize: "11px",
    color: "#525252",
  },
  main: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    minWidth: 0,
  },
};
