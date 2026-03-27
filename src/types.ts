export type MessageStatus = "in_progress" | "done" | "error";

export interface BridgeMessage {
  id: string;
  from: string;
  displaySource?: string;
  to: string;
  content: string;
  timestamp: number;
  type?: "task" | "review" | "result" | "question" | "system";
  replyTo?: string;
  priority?: "normal" | "urgent";
  status?: MessageStatus;
}

export type PermissionBehavior = "allow" | "deny";

export interface PermissionPrompt {
  agent: string;
  requestId: string;
  toolName: string;
  description: string;
  inputPreview?: string;
  createdAt: number;
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

/** Metadata returned from the Codex account/session integration. */
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
