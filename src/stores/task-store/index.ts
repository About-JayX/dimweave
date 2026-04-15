import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  Provider,
  ProviderHistoryInfo,
  ReplyTarget,
  SessionRole,
  TaskAgentInfo,
  TaskConfig,
  TaskProviderSummary,
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
  taskAgents?: TaskAgentInfo[];
  providerSummary?: TaskProviderSummary | null;
};

type TaskSetter = (
  fn: (state: {
    activeTaskId: string | null;
    selectedWorkspace: string | null;
    tasks: Record<string, TaskInfo>;
    taskAgents: Record<string, TaskAgentInfo[]>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
    providerSummaries: Record<string, TaskProviderSummary>;
    providerHistory: Record<string, ProviderHistoryInfo[]>;
    providerHistoryLoading: Record<string, boolean>;
    providerHistoryError: Record<string, string | null>;
    bootstrapComplete: boolean;
    bootstrapError: string | null;
  }) => Partial<{
    activeTaskId: string | null;
    selectedWorkspace: string | null;
    tasks: Record<string, TaskInfo>;
    taskAgents: Record<string, TaskAgentInfo[]>;
    sessions: Record<string, any[]>;
    artifacts: Record<string, any[]>;
    providerSummaries: Record<string, TaskProviderSummary>;
    providerHistory: Record<string, ProviderHistoryInfo[]>;
    providerHistoryLoading: Record<string, boolean>;
    providerHistoryError: Record<string, string | null>;
    bootstrapComplete: boolean;
    bootstrapError: string | null;
  }>,
) => void;

export function snapshotToPatch(snap: TaskSnapshot) {
  const summaryPatch: Record<string, TaskProviderSummary> =
    snap.providerSummary ? { [snap.task.taskId]: snap.providerSummary } : {};
  return {
    activeTaskId: snap.task.taskId,
    selectedWorkspace: snap.task.projectRoot,
    tasks: { [snap.task.taskId]: snap.task },
    taskAgents: { [snap.task.taskId]: snap.taskAgents ?? [] },
    sessions: { [snap.task.taskId]: snap.sessions },
    artifacts: { [snap.task.taskId]: snap.artifacts },
    providerSummaries: summaryPatch,
  };
}

export async function bootstrapTaskStore(
  set: TaskSetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
  listenImpl: typeof createTaskListeners = createTaskListeners,
  onActiveTaskChanged?: () => void,
) {
  set(() => ({ bootstrapComplete: false, bootstrapError: null }));

  try {
    await invokeImpl("daemon_clear_active_task");
    const snap = await invokeImpl<TaskSnapshot | null>("daemon_get_task_snapshot");
    if (snap) {
      set(() => snapshotToPatch(snap));
    }
    set(() => ({ bootstrapComplete: true }));
    unlisteners = await listenImpl(set as any, onActiveTaskChanged);
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

export function createLoadWorkspaceTasksAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  return async (workspace: string): Promise<void> => {
    const tasks = await invokeImpl<TaskInfo[]>("daemon_list_tasks", { workspace });
    set((s) => {
      const merged = { ...s.tasks };
      for (const t of tasks) merged[t.taskId] = t;
      return { tasks: merged };
    });
  };
}

export function createConfiguredTaskAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  return async (workspace: string, title: string, config: TaskConfig): Promise<TaskInfo> => {
    const task = await invokeImpl<TaskInfo>("daemon_create_task", {
      workspace,
      title,
      leadProvider: config.leadProvider,
      coderProvider: config.coderProvider,
    });
    set((s) => ({
      activeTaskId: task.taskId,
      tasks: { ...s.tasks, [task.taskId]: task },
    }));
    return task;
  };
}

export function createUpdateTaskConfigAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  return async (taskId: string, config: TaskConfig): Promise<TaskInfo> => {
    const task = await invokeImpl<TaskInfo>("daemon_update_task_config", {
      taskId,
      leadProvider: config.leadProvider,
      coderProvider: config.coderProvider,
    });
    set((s) => ({ tasks: { ...s.tasks, [task.taskId]: task } }));
    return task;
  };
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

export function setReplyTargetPatch(
  state: TaskStoreData,
  taskId: string | null,
  target: ReplyTarget,
): Partial<TaskStoreData> {
  if (!taskId) return {};
  return {
    replyTargets: {
      ...state.replyTargets,
      [taskId]: target,
    },
  };
}

export function createDeleteTaskAction(
  set: ProviderHistorySetter,
  invokeImpl: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> = invoke,
) {
  return async (taskId: string): Promise<void> => {
    await invokeImpl("daemon_delete_task", { taskId });
    set((s) => {
      const task = s.tasks[taskId];
      const workspace = task?.projectRoot ?? null;
      const { [taskId]: _t, ...remainingTasks } = s.tasks;
      const { [taskId]: _a, ...remainingAgents } = s.taskAgents;
      const { [taskId]: _s, ...remainingSessions } = s.sessions;
      const { [taskId]: _ar, ...remainingArtifacts } = s.artifacts;
      const { [taskId]: _ps, ...remainingSummaries } = s.providerSummaries;
      const { [taskId]: _rt, ...remainingTargets } = s.replyTargets;

      let nextActiveId: string | null = s.activeTaskId;
      if (s.activeTaskId === taskId) {
        const fallback = workspace
          ? Object.values(remainingTasks)
              .filter((t) => t.projectRoot === workspace)
              .sort((a, b) => b.createdAt - a.createdAt)[0]
          : undefined;
        nextActiveId = fallback?.taskId ?? null;
      }

      return {
        activeTaskId: nextActiveId,
        tasks: remainingTasks,
        taskAgents: remainingAgents,
        sessions: remainingSessions,
        artifacts: remainingArtifacts,
        providerSummaries: remainingSummaries,
        replyTargets: remainingTargets,
      };
    });
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
    void bootstrapTaskStore(
      set as any,
      undefined,
      undefined,
      () => void get().fetchSnapshot(),
    ).catch(() => {});
  }

  const fetchProviderHistory = createFetchProviderHistoryAction(set as any);
  const loadWorkspaceTasks = createLoadWorkspaceTasksAction(set as any);
  const configuredTask = createConfiguredTaskAction(set as any);
  const updateTaskConfig = createUpdateTaskConfigAction(set as any);
  const startWorkspaceTask = createStartWorkspaceTaskAction(set as any);
  const deleteTaskAction = createDeleteTaskAction(set as any);

  return {
    activeTaskId: null,
    selectedWorkspace: null,
    tasks: {},
    taskAgents: {},
    replyTargets: {},
    sessions: {},
    artifacts: {},
    providerSummaries: {},
    providerHistory: {},
    providerHistoryLoading: {},
    providerHistoryError: {},
    bootstrapComplete: false,
    bootstrapError: null,
    lastSave: null,

    setSelectedWorkspace: (workspace) => {
      set(() => ({ selectedWorkspace: workspace }));
      if (workspace) void loadWorkspaceTasks(workspace);
    },

    loadWorkspaceTasks,

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

    createConfiguredTask: configuredTask,
    updateTaskConfig,
    startWorkspaceTask,

    selectTask: async (taskId) => {
      await invoke("daemon_select_task", { taskId });
      // Refresh snapshot to get fresh provider summary for the new task
      await get().fetchSnapshot();
    },

    setReplyTarget: (target) =>
      set((s) => setReplyTargetPatch(s, s.activeTaskId, target)),

    fetchSnapshot: async () => {
      const snap = await invoke<TaskSnapshot | null>("daemon_get_task_snapshot");
      if (!snap) return;
      const patch = snapshotToPatch(snap);
      set((s) => ({
        ...patch,
        tasks: { ...s.tasks, [snap.task.taskId]: snap.task },
        taskAgents: { ...s.taskAgents, ...patch.taskAgents },
        sessions: { ...s.sessions, [snap.task.taskId]: snap.sessions },
        artifacts: { ...s.artifacts, [snap.task.taskId]: snap.artifacts },
        providerSummaries: { ...s.providerSummaries, ...patch.providerSummaries },
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
        await get().fetchProviderHistory(task.projectRoot);
      }
    },

    addTaskAgent: async (taskId, provider, role, displayName) => {
      const agent = await invoke<TaskAgentInfo>("daemon_add_task_agent", {
        taskId,
        provider,
        role,
        displayName: displayName ?? null,
      });
      return agent;
    },

    removeTaskAgent: async (agentId) => {
      await invoke("daemon_remove_task_agent", { agentId });
    },

    updateTaskAgent: async (agentId, provider, role, displayName) => {
      await invoke("daemon_update_task_agent", {
        agentId,
        provider,
        role,
        displayName: displayName ?? null,
      });
    },

    reorderTaskAgents: async (taskId, agentIds) => {
      await invoke("daemon_reorder_task_agents", { taskId, agentIds });
    },

    deleteTask: deleteTaskAction,

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
