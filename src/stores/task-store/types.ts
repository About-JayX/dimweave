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
export type ReplyTarget = "auto" | "lead" | "coder";
export type ArtifactKind =
  | "research"
  | "plan"
  | "review"
  | "diff"
  | "verification"
  | "summary";

export interface TaskInfo {
  taskId: string;
  workspaceRoot: string;
  title: string;
  status: TaskStatus;
  leadSessionId?: string | null;
  currentCoderSessionId?: string | null;
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

// Store data (separate from actions for testability)

export interface TaskStoreData {
  activeTaskId: string | null;
  tasks: Record<string, TaskInfo>;
  replyTargets: Record<string, ReplyTarget>;
  sessions: Record<string, SessionInfo[]>;
  artifacts: Record<string, ArtifactInfo[]>;
  providerHistory: Record<string, ProviderHistoryInfo[]>;
  providerHistoryLoading: Record<string, boolean>;
  providerHistoryError: Record<string, string | null>;
  bootstrapComplete: boolean;
  bootstrapError: string | null;
}

export interface TaskStoreState extends TaskStoreData {
  createTask: (workspace: string, title: string) => Promise<TaskInfo>;
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
  cleanup: () => void;
}
