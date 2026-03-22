import { useState, useCallback } from "react";

interface ReplyInputProps {
  onSend: (content: string) => void;
  disabled: boolean;
}

export function ReplyInput({ onSend, disabled }: ReplyInputProps) {
  const [text, setText] = useState("");

  const handleSend = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed) return;
    onSend(trimmed);
    setText("");
  }, [text, onSend]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  return (
    <div style={styles.container}>
      <textarea
        style={styles.textarea}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={disabled ? "Connect daemon to send messages..." : "Send message to Codex (Enter to send)"}
        disabled={disabled}
        rows={2}
      />
      <button
        style={{
          ...styles.sendBtn,
          opacity: disabled || !text.trim() ? 0.4 : 1,
        }}
        onClick={handleSend}
        disabled={disabled || !text.trim()}
      >
        Send
      </button>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: "flex",
    gap: "8px",
    padding: "12px 16px",
    borderTop: "1px solid #2d2d2d",
    alignItems: "flex-end",
  },
  textarea: {
    flex: 1,
    padding: "8px 12px",
    fontSize: "13px",
    backgroundColor: "#1e1e1e",
    color: "#e5e5e5",
    border: "1px solid #333",
    borderRadius: "6px",
    resize: "none",
    fontFamily: "inherit",
    outline: "none",
  },
  sendBtn: {
    padding: "8px 20px",
    fontSize: "13px",
    fontWeight: 500,
    backgroundColor: "#8b5cf6",
    color: "#fff",
    border: "none",
    borderRadius: "6px",
    cursor: "pointer",
    whiteSpace: "nowrap",
  },
};
