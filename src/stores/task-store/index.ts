import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  Provider,
  ProviderHistoryInfo,
  SessionRole,
  TaskInfo,
  TaskStoreState,
} from "./types";
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
    providerHistory: Record<string, ProviderHistoryInfo[]>;
  }) => Partial<{
    activeTaskId: string | null;
    tasks: Record<string, TaskInfo>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
    providerHistory: Record<string, ProviderHistoryInfo[]>;
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

export const useTaskStore = create<TaskStoreState>((set, get) => {
  if (typeof window !== "undefined") {
    void bootstrapTaskStore(set as any);
  }

  return {
    activeTaskId: null,
    tasks: {},
    sessions: {},
    artifacts: {},
    providerHistory: {},

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

    fetchProviderHistory: async (workspace) => {
      const entries = await invoke<ProviderHistoryInfo[]>(
        "daemon_list_provider_history",
        {
          workspace,
        },
      );
      set((s) => ({
        providerHistory: { ...s.providerHistory, [workspace]: entries },
      }));
    },

    resumeSession: async (sessionId) => {
      await invoke("daemon_resume_session", { sessionId });
    },

    attachProviderHistory: async (
      provider: Provider,
      externalId: string,
      cwd: string,
      role: SessionRole,
    ) => {
      await invoke("daemon_attach_provider_history", {
        provider,
        externalId,
        cwd,
        role,
      });
      await get().fetchSnapshot();
      const activeTaskId = get().activeTaskId;
      const task = activeTaskId ? get().tasks[activeTaskId] : null;
      if (task) {
        await get().fetchProviderHistory(task.workspaceRoot);
      }
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
