import { useEffect, useRef, useState, useCallback } from "react";
import type { GuiEvent, BridgeMessage, AgentInfo, DaemonStatus } from "../types";

const GUI_WS_URL = "ws://127.0.0.1:4503";
const RECONNECT_INTERVAL = 3000;

export function useBridgeWebSocket() {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [connected, setConnected] = useState(false);
  const [messages, setMessages] = useState<BridgeMessage[]>([]);
  const [agents, setAgents] = useState<Record<string, AgentInfo>>({
    claude: { name: "claude", displayName: "Claude Code", status: "disconnected" },
    codex: { name: "codex", displayName: "Codex", status: "disconnected" },
  });
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const ws = new WebSocket(GUI_WS_URL);

    ws.onopen = () => {
      setConnected(true);
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
    };

    ws.onmessage = (event) => {
      let guiEvent: GuiEvent;
      try {
        guiEvent = JSON.parse(event.data);
      } catch {
        return;
      }

      switch (guiEvent.type) {
        case "agent_message":
          setMessages((prev) => [...prev, guiEvent.payload as BridgeMessage]);
          break;

        case "agent_status": {
          const { agent, status, error, threadId } = guiEvent.payload;
          setAgents((prev) => ({
            ...prev,
            [agent]: {
              ...prev[agent],
              name: agent,
              displayName: prev[agent]?.displayName ?? agent,
              status,
              error,
              threadId,
            },
          }));
          break;
        }

        case "daemon_status":
          setDaemonStatus(guiEvent.payload as DaemonStatus);
          break;

        case "system_log":
          setMessages((prev) => [
            ...prev,
            {
              id: `log_${Date.now()}`,
              source: "system",
              content: guiEvent.payload.message,
              timestamp: guiEvent.timestamp,
            },
          ]);
          break;
      }
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;
      reconnectTimer.current = setTimeout(connect, RECONNECT_INTERVAL);
    };

    ws.onerror = () => {
      ws.close();
    };

    wsRef.current = ws;
  }, []);

  const sendToCodex = useCallback((content: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "send_to_codex", content }));
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const launchCodexTui = useCallback(() => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "launch_codex_tui" }));
  }, []);

  const stopCodexTui = useCallback(() => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "stop_codex_tui" }));
  }, []);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
    };
  }, [connect]);

  return { connected, messages, agents, daemonStatus, sendToCodex, clearMessages, launchCodexTui, stopCodexTui };
}
