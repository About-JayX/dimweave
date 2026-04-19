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

export interface UiError {
  id: number;
  message: string;
  componentStack?: string;
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
  uiErrors: UiError[];
  permissionPrompts: PermissionPrompt[];
  permissionError: PermissionResolutionError | null;
  runtimeHealth: RuntimeHealthInfo | null;
  claudeNeedsAttention: boolean;
  claudeRole: string;
  codexRole: string;
  /** Mirror of the active task's stream state. Legacy consumers read this. */
  claudeStream: ClaudeStreamState;
  /** Mirror of the active task's stream state. Legacy consumers read this. */
  codexStream: CodexStreamState;
  /**
   * Per-task stream buckets — source of truth. Stream events update the
   * specific task's bucket; task switch copies the matching bucket into
   * the singleton mirrors above. Avoids losing in-progress state when the
   * user navigates away and back during an agent's turn.
   */
  claudeStreamsByTask: Record<string, ClaudeStreamState>;
  codexStreamsByTask: Record<string, CodexStreamState>;
  draft: string;

  setDraft: (text: string) => void;
  clearClaudeAttention: () => void;
  sendToCodex: (
    message: string,
    target?: string,
    attachments?: Attachment[],
    taskId?: string,
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
    taskId?: string;
  }) => Promise<void>;
  pushUiError: (message: string, componentStack?: string) => void;
  clearUiErrors: () => void;
  setRole: (agent: "claude" | "codex", role: string) => void;
  cleanup: () => void;
}
