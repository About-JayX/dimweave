import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActiveTaskChangedPayload,
  ArtifactsChangedPayload,
  SaveStatus,
  SessionTreeChangedPayload,
  TaskInfo,
  TaskStoreData,
} from "./types";

type TaskSetter = (
  fn: (state: TaskStoreData) => Partial<TaskStoreData>,
) => void;

// ── Pure reducers (exported for testing) ─────────────────────

export function reduceTaskUpdated(
  state: TaskStoreData,
  task: TaskInfo,
): Partial<TaskStoreData> {
  return { tasks: { ...state.tasks, [task.taskId]: task } };
}

export function reduceActiveTaskChanged(
  _state: TaskStoreData,
  payload: ActiveTaskChangedPayload,
): Partial<TaskStoreData> {
  return { activeTaskId: payload.taskId };
}

export function reduceSessionTreeChanged(
  state: TaskStoreData,
  payload: SessionTreeChangedPayload,
): Partial<TaskStoreData> {
  return {
    sessions: { ...state.sessions, [payload.taskId]: payload.sessions },
  };
}

export function reduceArtifactsChanged(
  state: TaskStoreData,
  payload: ArtifactsChangedPayload,
): Partial<TaskStoreData> {
  return {
    artifacts: { ...state.artifacts, [payload.taskId]: payload.artifacts },
  };
}

export function reduceSaveStatus(
  _state: TaskStoreData,
  payload: SaveStatus,
): Partial<TaskStoreData> {
  return { lastSave: payload };
}

// ── Listener setup ───────────────────────────────────────────

export function createTaskListeners(set: TaskSetter): Promise<UnlistenFn[]> {
  return Promise.all([
    listen<TaskInfo>("task_updated", (e) => {
      set((s) => reduceTaskUpdated(s, e.payload));
    }),
    listen<ActiveTaskChangedPayload>("active_task_changed", (e) => {
      set((s) => reduceActiveTaskChanged(s, e.payload));
    }),
    listen<SessionTreeChangedPayload>("session_tree_changed", (e) => {
      set((s) => reduceSessionTreeChanged(s, e.payload));
    }),
    listen<ArtifactsChangedPayload>("artifacts_changed", (e) => {
      set((s) => reduceArtifactsChanged(s, e.payload));
    }),
    listen<SaveStatus>("task_save_status", (e) => {
      set((s) => reduceSaveStatus(s, e.payload));
    }),
  ]);
}
