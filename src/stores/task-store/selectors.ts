import type {
  ArtifactInfo,
  ProviderHistoryInfo,
  ReplyTarget,
  SessionInfo,
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
