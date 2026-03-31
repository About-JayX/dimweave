import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { TaskInfo, TaskStoreState } from "./types";
import { createTaskListeners } from "./events";

export type { TaskInfo, TaskStoreState } from "./types";

let unlisteners: (() => void)[] = [];

type TaskSnapshot = {
  task: TaskInfo;
  sessions: any[];
  artifacts: any[];
};

type TaskSetter = (
  fn: (state: {
    activeTaskId: string | null;
    tasks: Record<string, TaskInfo>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
  }) => Partial<{
    activeTaskId: string | null;
    tasks: Record<string, TaskInfo>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
  }>,
) => void;

export function snapshotToPatch(snap: TaskSnapshot) {
  return {
    activeTaskId: snap.task.taskId,
    tasks: { [snap.task.taskId]: snap.task },
    sessions: { [snap.task.taskId]: snap.sessions },
    artifacts: { [snap.task.taskId]: snap.artifacts },
  };
}

export async function bootstrapTaskStore(
  set: TaskSetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
  listenImpl: typeof createTaskListeners = createTaskListeners,
) {
  const snap = await invokeImpl<TaskSnapshot | null>("daemon_get_task_snapshot");
  if (snap) {
    set(() => snapshotToPatch(snap));
  }
  unlisteners = await listenImpl(set as any);
}

export const useTaskStore = create<TaskStoreState>((set) => {
  if (typeof window !== "undefined") {
    void bootstrapTaskStore(set as any);
  }

  return {
    activeTaskId: null,
    tasks: {},
    sessions: {},
    artifacts: {},

    createTask: async (workspace, title) => {
      const task = await invoke<TaskInfo>("daemon_create_task", {
        workspace,
        title,
      });
      set((s) => ({ tasks: { ...s.tasks, [task.taskId]: task } }));
      return task;
    },

    selectTask: async (taskId) => {
      await invoke("daemon_select_task", { taskId });
    },

    approveReview: async () => {
      await invoke("daemon_approve_review");
    },

    fetchSnapshot: async () => {
      const snap = await invoke<TaskSnapshot | null>("daemon_get_task_snapshot");
      if (!snap) return;
      set((s) => ({
        ...snapshotToPatch(snap),
        tasks: { ...s.tasks, [snap.task.taskId]: snap.task },
        sessions: { ...s.sessions, [snap.task.taskId]: snap.sessions },
        artifacts: { ...s.artifacts, [snap.task.taskId]: snap.artifacts },
      }));
    },

    resumeSession: async (sessionId) => {
      await invoke("daemon_resume_session", { sessionId });
    },

    cleanup: () => {
      for (const fn of unlisteners) fn();
      unlisteners = [];
    },
  };
});

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    useTaskStore.getState().cleanup();
  });
}
