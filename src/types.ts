export type MessageSource = "claude" | "codex" | "system";

export interface BridgeMessage {
  id: string;
  source: MessageSource;
  content: string;
  timestamp: number;
}

export type AgentStatus = "disconnected" | "connecting" | "connected" | "error";

export interface AgentInfo {
  name: string;
  displayName: string;
  status: AgentStatus;
  error?: string;
  threadId?: string;
}

export interface DaemonStatus {
  bridgeReady: boolean;
  tuiConnected: boolean;
  threadId: string | null;
  queuedMessageCount: number;
  proxyUrl: string;
  appServerUrl: string;
  pid: number;
  codexBootstrapped: boolean;
  codexTuiRunning: boolean;
  claudeConnected: boolean;
}

export interface GuiEvent {
  type: "agent_message" | "agent_status" | "system_log" | "daemon_status";
  payload: any;
  timestamp: number;
}
