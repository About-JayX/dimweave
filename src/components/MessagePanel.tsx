import { useEffect, useRef } from "react";
import type { BridgeMessage } from "../types";

interface MessagePanelProps {
  messages: BridgeMessage[];
  onClear: () => void;
}

const sourceColors: Record<string, string> = {
  claude: "#8b5cf6",
  codex: "#22c55e",
  system: "#6b7280",
};

const sourceLabels: Record<string, string> = {
  claude: "Claude",
  codex: "Codex",
  system: "System",
};

export function MessagePanel({ messages, onClear }: MessagePanelProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h3 style={styles.title}>Messages ({messages.length})</h3>
        <button style={styles.clearBtn} onClick={onClear}>Clear</button>
      </div>

      <div style={styles.messageList}>
        {messages.length === 0 && (
          <div style={styles.empty}>
            No messages yet. Connect Claude Code and Codex to start bridging.
          </div>
        )}

        {messages.map((msg) => (
          <div key={msg.id} style={styles.messageRow}>
            <div style={styles.messageMeta}>
              <span style={{
                ...styles.sourceTag,
                backgroundColor: `${sourceColors[msg.source] ?? "#6b7280"}20`,
                color: sourceColors[msg.source] ?? "#6b7280",
                borderColor: `${sourceColors[msg.source] ?? "#6b7280"}40`,
              }}>
                {sourceLabels[msg.source] ?? msg.source}
              </span>
              <span style={styles.timestamp}>
                {new Date(msg.timestamp).toLocaleTimeString()}
              </span>
            </div>
            <div style={styles.messageContent}>{msg.content}</div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: "flex",
    flexDirection: "column",
    flex: 1,
    minHeight: 0,
  },
  header: {
    display: "flex",
    alignItems: "center",
    padding: "12px 16px",
    borderBottom: "1px solid #2d2d2d",
  },
  title: {
    margin: 0,
    fontSize: "14px",
    fontWeight: 600,
    color: "#e5e5e5",
    flex: 1,
  },
  clearBtn: {
    padding: "4px 12px",
    fontSize: "12px",
    backgroundColor: "#2d2d2d",
    color: "#a3a3a3",
    border: "1px solid #404040",
    borderRadius: "4px",
    cursor: "pointer",
  },
  messageList: {
    flex: 1,
    overflowY: "auto",
    padding: "8px 16px",
  },
  empty: {
    padding: "40px 0",
    textAlign: "center",
    color: "#525252",
    fontSize: "13px",
  },
  messageRow: {
    padding: "10px 0",
    borderBottom: "1px solid #1e1e1e",
  },
  messageMeta: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    marginBottom: "4px",
  },
  sourceTag: {
    fontSize: "11px",
    fontWeight: 600,
    padding: "2px 8px",
    borderRadius: "4px",
    border: "1px solid",
    textTransform: "uppercase" as const,
  },
  timestamp: {
    fontSize: "11px",
    color: "#525252",
    fontFamily: "monospace",
  },
  messageContent: {
    fontSize: "13px",
    color: "#d4d4d4",
    lineHeight: 1.5,
    whiteSpace: "pre-wrap",
    wordBreak: "break-word",
  },
};
