import type {
  AgentInfo,
  Attachment,
  BridgeMessage,
  PermissionBehavior,
  PermissionPrompt,
  PermissionResolutionError,
  RuntimeHealthInfo,
} from "@/types";

export interface TerminalLine {
  id: number;
  agent: string;
  kind: "text" | "error";
  line: string;
  timestamp: number;
}

export interface CodexStreamState {
  thinking: boolean;
  currentDelta: string;
  lastMessage: string;
  turnStatus: string;
  activity: string;
  reasoning: string;
  commandOutput: string;
}

export type ClaudeBlockType = "thinking" | "text" | "tool" | "idle";

export interface ClaudeStreamState {
  thinking: boolean;
  previewText: string;
  thinkingText: string;
  blockType: ClaudeBlockType;
  toolName: string;
  lastUpdatedAt: number;
}

export interface BridgeState {
  connected: boolean;
  messages: BridgeMessage[];
  agents: Record<string, AgentInfo>;
  terminalLines: TerminalLine[];
  permissionPrompts: PermissionPrompt[];
  permissionError: PermissionResolutionError | null;
  runtimeHealth: RuntimeHealthInfo | null;
  claudeNeedsAttention: boolean;
  claudeRole: string;
  codexRole: string;
  claudeStream: ClaudeStreamState;
  codexStream: CodexStreamState;
  draft: string;

  setDraft: (text: string) => void;
  clearClaudeAttention: () => void;
  sendToCodex: (
    content: string,
    target?: string,
    attachments?: Attachment[],
  ) => void;
  clearMessages: () => void;
  stopCodexTui: () => void;
  respondToPermission: (
    requestId: string,
    behavior: PermissionBehavior,
  ) => Promise<void>;
  applyConfig: (config: {
    model?: string;
    reasoningEffort?: string;
    cwd?: string;
    resumeThreadId?: string;
  }) => Promise<void>;
  setRole: (agent: "claude" | "codex", role: string) => void;
  cleanup: () => void;
}
