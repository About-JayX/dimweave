export interface BridgeMessage {
  id: string;
  from: string;
  to: string;
  content: string;
  timestamp: number;
  type?: "task" | "review" | "result" | "question" | "system";
  replyTo?: string;
  priority?: "normal" | "urgent";
}

export type AgentStatus = "disconnected" | "connecting" | "connected" | "error";

export interface AgentInfo {
  name: string;
  displayName: string;
  status: AgentStatus;
  error?: string;
  threadId?: string;
}

export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

/** Protocol-derived data from daemon WS (model, reasoning, tokens). */
export interface CodexAccountInfo {
  initialized: boolean;
  userAgent?: string;
  platformOs?: string;
  platformFamily?: string;
  model?: string;
  modelProvider?: string;
  serviceTier?: string;
  reasoningEffort?: string;
  cwd?: string;
  approvalPolicy?: string;
  sandbox?: string;
  planType?: string;
  usage?: TokenUsage;
  cumulativeUsage?: TokenUsage;
  lastUpdated: number;
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
  codexAccount?: CodexAccountInfo;
  claudeRole?: string;
  codexRole?: string;
}

export interface GuiEvent {
  type:
    | "agent_message"
    | "agent_message_started"
    | "agent_message_delta"
    | "codex_phase"
    | "terminal_output"
    | "claude_rate_limit"
    | "pty_inject"
    | "agent_status"
    | "role_sync"
    | "system_log"
    | "daemon_status";
  payload: any;
  timestamp: number;
}
