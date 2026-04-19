// Domain types mirroring Rust task_graph::types (camelCase fields, snake_case enum values)

export type TaskStatus =
  | "draft"
  | "planning"
  | "implementing"
  | "reviewing"
  | "done"
  | "error";

export type SessionStatus = "active" | "paused" | "completed" | "error";
export type Provider = "claude" | "codex";
export type SessionRole = "lead" | "coder";
export type ReplyTarget = string;
export type ArtifactKind =
  | "research"
  | "plan"
  | "review"
  | "diff"
  | "verification"
  | "summary";

export interface TaskInfo {
  taskId: string;
  projectRoot: string;
  taskWorktreeRoot: string;
  /** @deprecated Use projectRoot. Backend no longer sends this field. */
  workspaceRoot?: string;
  title: string;
  status: TaskStatus;
  leadSessionId?: string | null;
  currentCoderSessionId?: string | null;
  leadProvider: Provider;
  coderProvider: Provider;
  createdAt: number;
  updatedAt: number;
}

export interface SessionInfo {
  sessionId: string;
  taskId: string;
  parentSessionId?: string | null;
  provider: Provider;
  role: SessionRole;
  externalSessionId?: string | null;
  transcriptPath?: string | null;
  status: SessionStatus;
  cwd: string;
  title: string;
  createdAt: number;
  updatedAt: number;
}

export interface ProviderHistoryInfo {
  provider: Provider;
  externalId: string;
  title?: string | null;
  preview?: string | null;
  cwd?: string | null;
  archived: boolean;
  createdAt: number;
  updatedAt: number;
  status: SessionStatus;
  normalizedSessionId?: string | null;
  normalizedTaskId?: string | null;
}

export interface ArtifactInfo {
  artifactId: string;
  taskId: string;
  sessionId: string;
  kind: ArtifactKind;
  title: string;
  contentRef: string;
  createdAt: number;
}

export interface TaskAgentInfo {
  agentId: string;
  taskId: string;
  provider: Provider;
  role: string;
  displayName?: string | null;
  model?: string | null;
  effort?: string | null;
  order: number;
  createdAt: number;
}

export interface TaskProviderSessionInfo {
  provider: "claude" | "codex";
  externalSessionId: string;
  cwd: string;
  connectionMode: "new" | "resumed";
}

export interface TaskProviderSummary {
  taskId: string;
  leadProvider: string;
  coderProvider: string;
  leadOnline: boolean;
  coderOnline: boolean;
  leadProviderSession?: TaskProviderSessionInfo | null;
  coderProviderSession?: TaskProviderSessionInfo | null;
}

export interface AgentRuntimeStatus {
  agentId: string;
  online: boolean;
}

// Event payloads from gui_task.rs

export interface ActiveTaskChangedPayload {
  taskId: string | null;
}

export interface SessionTreeChangedPayload {
  taskId: string;
  sessions: SessionInfo[];
}

export interface ArtifactsChangedPayload {
  taskId: string;
  artifacts: ArtifactInfo[];
}

export interface TaskAgentsChangedPayload {
  taskId: string;
  agents: TaskAgentInfo[];
}

export interface AgentRuntimeStatusChangedPayload {
  taskId: string;
  statuses: AgentRuntimeStatus[];
}

// Store data (separate from actions for testability)

export interface SaveStatus {
  success: boolean;
  error?: string | null;
  taskId: string;
  timestamp: number;
}

export interface TaskStoreData {
  activeTaskId: string | null;
  selectedWorkspace: string | null;
  tasks: Record<string, TaskInfo>;
  taskAgents: Record<string, TaskAgentInfo[]>;
  replyTargets: Record<string, ReplyTarget>;
  sessions: Record<string, SessionInfo[]>;
  artifacts: Record<string, ArtifactInfo[]>;
  providerSummaries: Record<string, TaskProviderSummary>;
  agentRuntimeStatuses: Record<string, AgentRuntimeStatus[]>;
  providerHistory: Record<string, ProviderHistoryInfo[]>;
  providerHistoryLoading: Record<string, boolean>;
  providerHistoryError: Record<string, string | null>;
  bootstrapComplete: boolean;
  bootstrapError: string | null;
  lastSave: SaveStatus | null;
}

export interface TaskConfig {
  leadProvider: Provider;
  coderProvider: Provider;
}

export interface TaskStoreState extends TaskStoreData {
  setSelectedWorkspace: (workspace: string | null) => void;
  loadWorkspaceTasks: (workspace: string) => Promise<void>;
  createTask: (workspace: string, title: string) => Promise<TaskInfo>;
  createConfiguredTask: (
    workspace: string,
    title: string,
    config: TaskConfig,
  ) => Promise<TaskInfo>;
  updateTaskConfig: (taskId: string, config: TaskConfig) => Promise<TaskInfo>;
  startWorkspaceTask: (workspace: string) => Promise<TaskInfo>;
  selectTask: (taskId: string) => Promise<void>;
  setReplyTarget: (target: ReplyTarget) => void;
  fetchSnapshot: () => Promise<void>;
  fetchProviderHistory: (workspace: string) => Promise<void>;
  resumeSession: (sessionId: string) => Promise<void>;
  attachProviderHistory: (
    provider: Provider,
    externalId: string,
    cwd: string,
    role: SessionRole,
  ) => Promise<void>;
  addTaskAgent: (
    taskId: string,
    provider: Provider,
    role: string,
    displayName?: string | null,
    model?: string | null,
    effort?: string | null,
  ) => Promise<TaskAgentInfo>;
  removeTaskAgent: (agentId: string) => Promise<void>;
  stopAgent: (agentId: string) => Promise<void>;
  updateTaskAgent: (
    agentId: string,
    provider: Provider,
    role: string,
    displayName?: string | null,
    model?: string | null,
    effort?: string | null,
  ) => Promise<void>;
  reorderTaskAgents: (taskId: string, agentIds: string[]) => Promise<void>;
  deleteTask: (taskId: string) => Promise<void>;
  cleanup: () => void;
}
