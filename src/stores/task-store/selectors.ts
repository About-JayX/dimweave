import type {
  ArtifactInfo,
  Provider,
  ProviderHistoryInfo,
  SessionInfo,
  TaskAgentInfo,
  TaskInfo,
  TaskProviderSessionInfo,
  TaskProviderSummary,
  TaskStoreState,
} from "./types";

const EMPTY_SESSIONS: SessionInfo[] = [];
const EMPTY_ARTIFACTS: ArtifactInfo[] = [];
const EMPTY_PROVIDER_HISTORY: ProviderHistoryInfo[] = [];
const EMPTY_AGENTS: TaskAgentInfo[] = [];
const AUTO_ONLY: string[] = ["auto"];

export function selectActiveTask(state: TaskStoreState) {
  return state.activeTaskId ? (state.tasks[state.activeTaskId] ?? null) : null;
}

export function selectActiveReplyTarget(state: TaskStoreState): string {
  if (!state.activeTaskId) return "auto";
  return (
    state.replyTargets[state.activeTaskId] ?? selectDefaultReplyTarget(state)
  );
}

export function selectActiveTaskSessions(state: TaskStoreState) {
  return state.activeTaskId
    ? (state.sessions[state.activeTaskId] ?? EMPTY_SESSIONS)
    : EMPTY_SESSIONS;
}

export function selectActiveTaskArtifacts(state: TaskStoreState) {
  return state.activeTaskId
    ? (state.artifacts[state.activeTaskId] ?? EMPTY_ARTIFACTS)
    : EMPTY_ARTIFACTS;
}

export function selectActiveTaskSessionCount(state: TaskStoreState) {
  return state.activeTaskId
    ? (state.sessions[state.activeTaskId] ?? []).length
    : 0;
}

export function selectActiveTaskArtifactCount(state: TaskStoreState) {
  return state.activeTaskId
    ? (state.artifacts[state.activeTaskId] ?? []).length
    : 0;
}

export interface TaskProviderBindings {
  leadProvider: Provider;
  coderProvider: Provider;
  leadOnline: boolean;
  coderOnline: boolean;
  leadProviderSession?: TaskProviderSessionInfo | null;
  coderProviderSession?: TaskProviderSessionInfo | null;
}

const DEFAULT_BINDINGS: TaskProviderBindings = {
  leadProvider: "claude",
  coderProvider: "codex",
  leadOnline: false,
  coderOnline: false,
};

// Memoization cache — stable reference prevents Zustand infinite re-renders.
let _prevTaskId: string | null = null;
let _prevTask: TaskInfo | null | undefined = undefined;
let _prevSummary: TaskProviderSummary | undefined = undefined;
let _prevResult: TaskProviderBindings = DEFAULT_BINDINGS;

export function selectActiveTaskProviderBindings(
  state: TaskStoreState,
): TaskProviderBindings {
  const task = state.activeTaskId ? state.tasks[state.activeTaskId] : null;
  if (!task) return DEFAULT_BINDINGS;
  const summary = state.providerSummaries[task.taskId];
  if (
    state.activeTaskId === _prevTaskId &&
    task === _prevTask &&
    summary === _prevSummary
  ) {
    return _prevResult;
  }
  _prevTaskId = state.activeTaskId;
  _prevTask = task;
  _prevSummary = summary;
  _prevResult = {
    leadProvider: (summary?.leadProvider as Provider) ?? task.leadProvider,
    coderProvider: (summary?.coderProvider as Provider) ?? task.coderProvider,
    leadOnline: summary?.leadOnline ?? false,
    coderOnline: summary?.coderOnline ?? false,
    leadProviderSession: summary?.leadProviderSession ?? null,
    coderProviderSession: summary?.coderProviderSession ?? null,
  };
  return _prevResult;
}

export function selectProviderSummary(
  state: TaskStoreState,
): TaskProviderSummary | null {
  if (!state.activeTaskId) return null;
  return state.providerSummaries[state.activeTaskId] ?? null;
}

export function makeProviderHistorySelector(
  workspace: string | null | undefined,
) {
  return (state: TaskStoreState) =>
    workspace
      ? (state.providerHistory[workspace] ?? EMPTY_PROVIDER_HISTORY)
      : EMPTY_PROVIDER_HISTORY;
}

export function makeProviderHistoryLoadingSelector(
  workspace: string | null | undefined,
) {
  return (state: TaskStoreState) =>
    workspace ? (state.providerHistoryLoading[workspace] ?? false) : false;
}

export function makeProviderHistoryErrorSelector(
  workspace: string | null | undefined,
) {
  return (state: TaskStoreState) =>
    workspace ? (state.providerHistoryError[workspace] ?? null) : null;
}

export function selectActiveTaskAgents(state: TaskStoreState): TaskAgentInfo[] {
  return state.activeTaskId
    ? (state.taskAgents[state.activeTaskId] ?? EMPTY_AGENTS)
    : EMPTY_AGENTS;
}

// Memoization for role options
let _roleOptsPrevAgents: TaskAgentInfo[] | undefined;
let _roleOptsPrevResult: string[] = AUTO_ONLY;

export function selectActiveTaskRoleOptions(state: TaskStoreState): string[] {
  const agents = selectActiveTaskAgents(state);
  if (agents === _roleOptsPrevAgents) return _roleOptsPrevResult;
  _roleOptsPrevAgents = agents;
  if (agents.length === 0) {
    _roleOptsPrevResult = AUTO_ONLY;
    return AUTO_ONLY;
  }
  const sorted = [...agents].sort((a, b) => a.order - b.order);
  const seen = new Set<string>();
  const roles: string[] = ["auto"];
  for (const a of sorted) {
    if (!seen.has(a.role)) {
      seen.add(a.role);
      roles.push(a.role);
    }
  }
  _roleOptsPrevResult = roles;
  return roles;
}

// Memoization for workspace task list (newest-first ordering for accordion)
const EMPTY_TASKS: TaskInfo[] = [];
let _wsPrevWorkspace: string | null | undefined;
let _wsPrevTasks: Record<string, TaskInfo> | undefined;
let _wsPrevResult: TaskInfo[] = EMPTY_TASKS;

export function selectWorkspaceTasks(state: TaskStoreState): TaskInfo[] {
  const ws = state.selectedWorkspace;
  if (!ws) return EMPTY_TASKS;
  if (ws === _wsPrevWorkspace && state.tasks === _wsPrevTasks)
    return _wsPrevResult;
  _wsPrevWorkspace = ws;
  _wsPrevTasks = state.tasks;
  const filtered = Object.values(state.tasks)
    .filter((t) => t.projectRoot === ws)
    .sort((a, b) => b.createdAt - a.createdAt);
  _wsPrevResult = filtered.length > 0 ? filtered : EMPTY_TASKS;
  return _wsPrevResult;
}

export function selectDefaultReplyTarget(state: TaskStoreState): string {
  const roles = selectActiveTaskRoleOptions(state);
  if (roles.includes("lead")) return "lead";
  return roles.length > 1 ? roles[1] : "auto";
}
