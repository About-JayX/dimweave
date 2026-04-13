import type {
  ArtifactInfo,
  Provider,
  ProviderHistoryInfo,
  ReplyTarget,
  SessionInfo,
  TaskProviderSessionInfo,
  TaskProviderSummary,
  TaskStoreState,
} from "./types";

const EMPTY_SESSIONS: SessionInfo[] = [];
const EMPTY_ARTIFACTS: ArtifactInfo[] = [];
const EMPTY_PROVIDER_HISTORY: ProviderHistoryInfo[] = [];

export function selectActiveTask(state: TaskStoreState) {
  return state.activeTaskId ? state.tasks[state.activeTaskId] ?? null : null;
}

export function selectActiveReplyTarget(state: TaskStoreState): ReplyTarget {
  return state.activeTaskId
    ? state.replyTargets[state.activeTaskId] ?? "auto"
    : "auto";
}

export function selectActiveTaskSessions(state: TaskStoreState) {
  return state.activeTaskId
    ? state.sessions[state.activeTaskId] ?? EMPTY_SESSIONS
    : EMPTY_SESSIONS;
}

export function selectActiveTaskArtifacts(state: TaskStoreState) {
  return state.activeTaskId
    ? state.artifacts[state.activeTaskId] ?? EMPTY_ARTIFACTS
    : EMPTY_ARTIFACTS;
}

export function selectActiveTaskSessionCount(state: TaskStoreState) {
  return state.activeTaskId ? (state.sessions[state.activeTaskId] ?? []).length : 0;
}

export function selectActiveTaskArtifactCount(state: TaskStoreState) {
  return state.activeTaskId ? (state.artifacts[state.activeTaskId] ?? []).length : 0;
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

export function selectActiveTaskProviderBindings(
  state: TaskStoreState,
): TaskProviderBindings {
  const task = state.activeTaskId ? state.tasks[state.activeTaskId] : null;
  if (!task) return DEFAULT_BINDINGS;
  const summary = state.providerSummaries[task.taskId];
  return {
    leadProvider: task.leadProvider,
    coderProvider: task.coderProvider,
    leadOnline: summary?.leadOnline ?? false,
    coderOnline: summary?.coderOnline ?? false,
    leadProviderSession: summary?.leadProviderSession ?? null,
    coderProviderSession: summary?.coderProviderSession ?? null,
  };
}

export function selectProviderSummary(
  state: TaskStoreState,
): TaskProviderSummary | null {
  if (!state.activeTaskId) return null;
  return state.providerSummaries[state.activeTaskId] ?? null;
}

export function makeProviderHistorySelector(workspace: string | null | undefined) {
  return (state: TaskStoreState) =>
    workspace ? state.providerHistory[workspace] ?? EMPTY_PROVIDER_HISTORY : EMPTY_PROVIDER_HISTORY;
}

export function makeProviderHistoryLoadingSelector(
  workspace: string | null | undefined,
) {
  return (state: TaskStoreState) =>
    workspace ? state.providerHistoryLoading[workspace] ?? false : false;
}

export function makeProviderHistoryErrorSelector(
  workspace: string | null | undefined,
) {
  return (state: TaskStoreState) =>
    workspace ? state.providerHistoryError[workspace] ?? null : null;
}
