import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  Provider,
  ProviderHistoryInfo,
  SessionRole,
  TaskStoreData,
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
    providerHistoryLoading: Record<string, boolean>;
    providerHistoryError: Record<string, string | null>;
    bootstrapComplete: boolean;
    bootstrapError: string | null;
  }) => Partial<{
    activeTaskId: string | null;
    tasks: Record<string, TaskInfo>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
    providerHistory: Record<string, ProviderHistoryInfo[]>;
    providerHistoryLoading: Record<string, boolean>;
    providerHistoryError: Record<string, string | null>;
    bootstrapComplete: boolean;
    bootstrapError: string | null;
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
  set(() => ({ bootstrapComplete: false, bootstrapError: null }));

  try {
    await invokeImpl("daemon_clear_active_task");
    const snap = await invokeImpl<TaskSnapshot | null>("daemon_get_task_snapshot");
    if (snap) {
      set(() => snapshotToPatch(snap));
    }
    set(() => ({ bootstrapComplete: true }));
    unlisteners = await listenImpl(set as any);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    set(() => ({ bootstrapComplete: false, bootstrapError: message }));
    throw error;
  }
}

type ProviderHistorySetter = (
  fn: (state: TaskStoreData) => Partial<TaskStoreData>,
) => void;

export function deriveWorkspaceTaskTitle(workspace: string) {
  const parts = workspace.split(/[\\/]/).filter(Boolean);
  return parts.at(-1) || workspace;
}

export function createStartWorkspaceTaskAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  return async (workspace: string): Promise<TaskInfo> => {
    const task = await invokeImpl<TaskInfo>("daemon_create_task", {
      workspace,
      title: deriveWorkspaceTaskTitle(workspace),
    });
    set((s) => ({
      activeTaskId: task.taskId,
      tasks: { ...s.tasks, [task.taskId]: task },
    }));
    return task;
  };
}

export function createFetchProviderHistoryAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  const inFlight = new Map<string, Promise<ProviderHistoryInfo[]>>();

  return async (workspace: string): Promise<void> => {
    const existing = inFlight.get(workspace);
    if (existing) {
      await existing;
      return;
    }

    set((s) => ({
      providerHistoryLoading: { ...s.providerHistoryLoading, [workspace]: true },
      providerHistoryError: { ...s.providerHistoryError, [workspace]: null },
    }));

    const request = invokeImpl<ProviderHistoryInfo[]>(
      "daemon_list_provider_history",
      {
        workspace,
      },
    )
      .then((entries) => {
        set((s) => ({
          providerHistory: { ...s.providerHistory, [workspace]: entries },
          providerHistoryLoading: {
            ...s.providerHistoryLoading,
            [workspace]: false,
          },
          providerHistoryError: { ...s.providerHistoryError, [workspace]: null },
        }));
        return entries;
      })
      .catch((error) => {
        set((s) => ({
          providerHistoryLoading: {
            ...s.providerHistoryLoading,
            [workspace]: false,
          },
          providerHistoryError: {
            ...s.providerHistoryError,
            [workspace]: error instanceof Error ? error.message : String(error),
          },
        }));
        throw error;
      })
      .finally(() => {
        inFlight.delete(workspace);
      });

    inFlight.set(workspace, request);
    await request;
  };
}

export const useTaskStore = create<TaskStoreState>((set, get) => {
  if (typeof window !== "undefined") {
    void bootstrapTaskStore(set as any).catch(() => {});
  }

  const fetchProviderHistory = createFetchProviderHistoryAction(set as any);
  const startWorkspaceTask = createStartWorkspaceTaskAction(set as any);

  return {
    activeTaskId: null,
    tasks: {},
    sessions: {},
    artifacts: {},
    providerHistory: {},
    providerHistoryLoading: {},
    providerHistoryError: {},
    bootstrapComplete: false,
    bootstrapError: null,

    createTask: async (workspace, title) => {
      const task = await invoke<TaskInfo>("daemon_create_task", {
        workspace,
        title,
      });
      set((s) => ({
        activeTaskId: task.taskId,
        tasks: { ...s.tasks, [task.taskId]: task },
      }));
      return task;
    },

    startWorkspaceTask,

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

    fetchProviderHistory,

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
