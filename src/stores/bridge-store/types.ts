import type {
  AgentInfo,
  BridgeMessage,
  PermissionBehavior,
  PermissionPrompt,
} from "@/types";

export interface TerminalLine {
  id: number;
  agent: string;
  kind: "text" | "error";
  line: string;
  timestamp: number;
}

export interface ClaudeTerminalChunk {
  id: number;
  data: string;
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

export interface ClaudeStreamState {
  thinking: boolean;
  previewText: string;
  lastUpdatedAt: number;
}

export interface BridgeState {
  connected: boolean;
  messages: BridgeMessage[];
  agents: Record<string, AgentInfo>;
  terminalLines: TerminalLine[];
  claudeTerminalChunks: ClaudeTerminalChunk[];
  claudeTerminalRunning: boolean;
  claudeTerminalExitCode?: number;
  claudeTerminalDetail?: string;
  permissionPrompts: PermissionPrompt[];
  claudeNeedsAttention: boolean;
  claudeFocusNonce: number;
  claudeRole: string;
  codexRole: string;
  claudeStream: ClaudeStreamState;
  codexStream: CodexStreamState;
  draft: string;

  setDraft: (text: string) => void;
  clearClaudeAttention: () => void;
  sendToCodex: (content: string, target?: string) => void;
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
