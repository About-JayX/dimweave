export type MessageStatus = "in_progress" | "done" | "error";
export type ProviderConnectionMode = "new" | "resumed";
export type RuntimeHealthLevel = "warning" | "error";

export interface ProviderSessionInfo {
  provider: "claude" | "codex";
  externalSessionId: string;
  cwd: string;
  connectionMode: ProviderConnectionMode;
}

export interface RuntimeHealthInfo {
  level: RuntimeHealthLevel;
  source: string;
  message: string;
}

export interface Attachment {
  filePath: string;
  fileName: string;
  isImage?: boolean;
  mediaType?: string;
}

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
  senderAgentId?: string;
  attachments?: Attachment[];
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

export interface PermissionResolutionError {
  requestId: string;
  message: string;
}

export type AgentStatus = "disconnected" | "connecting" | "connected" | "error";

export interface AgentInfo {
  name: string;
  displayName: string;
  status: AgentStatus;
  error?: string;
  threadId?: string;
  providerSession?: ProviderSessionInfo;
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
