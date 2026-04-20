import type {
  AgentInfo,
  Attachment,
  BridgeMessage,
  PermissionBehavior,
  PermissionPrompt,
  PermissionResolutionError,
  RuntimeHealthInfo,
} from "@/types";

/// Bucket key used when a message has no `task_id` (e.g. legacy system
/// diagnostic). Exported so listener-setup + selectors agree.
export const GLOBAL_MESSAGE_BUCKET = "__global__";

/// Max retained messages per task bucket in memory. Matches prior flat
/// `messages.slice(-999)` budget; Virtuoso virtualizes viewport so this
/// only caps long-session growth, not render cost.
export const MAX_MESSAGES_PER_BUCKET = 1000;

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
  /**
   * Chat messages bucketed by task_id. Primary storage; no flat aggregate
   * array is kept. Per-task lookup is O(1) and keeps sibling tasks'
   * references stable when one bucket mutates — so MessageList only
   * re-renders for its own task, and useMemo filter chains don't
   * invalidate every time any task receives a new message.
   * Messages without a task_id (system diagnostics that pre-date task
   * scoping) land in the `GLOBAL_MESSAGE_BUCKET` key.
   */
  messagesByTask: Record<string, BridgeMessage[]>;
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
