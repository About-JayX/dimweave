import type { TaskStoreState } from "./types";

export function selectActiveTask(state: TaskStoreState) {
  return state.activeTaskId ? state.tasks[state.activeTaskId] ?? null : null;
}

export function selectActiveTaskSessions(state: TaskStoreState) {
  return state.activeTaskId ? state.sessions[state.activeTaskId] ?? [] : [];
}

export function selectActiveTaskArtifacts(state: TaskStoreState) {
  return state.activeTaskId ? state.artifacts[state.activeTaskId] ?? [] : [];
}
