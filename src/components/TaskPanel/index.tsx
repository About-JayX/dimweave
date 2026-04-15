import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buildCodexLaunchConfig } from "@/components/AgentStatus/codex-launch-config";
import { buildClaudeLaunchRequest } from "@/components/ClaudePanel/launch-request";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskAgents,
  selectWorkspaceTasks,
} from "@/stores/task-store/selectors";
import { TaskHeader, type ReviewBadge } from "./TaskHeader";
import {
  TaskSetupDialog,
  type TaskSetupMode,
  type TaskSetupSubmitPayload,
} from "./TaskSetupDialog";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { getTaskPanelEmptyStateMessage } from "./view-model";

export function TaskPanel() {
  const task = useTaskStore(selectActiveTask);
  const workspaceTasks = useTaskStore(selectWorkspaceTasks);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectTask = useTaskStore((s) => s.selectTask);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const selectedWorkspace = useTaskStore((s) => s.selectedWorkspace);
  const agents = useTaskStore(selectActiveTaskAgents);
  const createTask = useTaskStore((s) => s.createTask);
  const addTaskAgent = useTaskStore((s) => s.addTaskAgent);
  const removeTaskAgent = useTaskStore((s) => s.removeTaskAgent);
  const updateTaskAgent = useTaskStore((s) => s.updateTaskAgent);
  const reorderTaskAgents = useTaskStore((s) => s.reorderTaskAgents);
  const deleteTask = useTaskStore((s) => s.deleteTask);
  const applyConfig = useBridgeStore((s) => s.applyConfig);
  const codexModels = useCodexAccountStore((s) => s.models);
  const fetchCodexModels = useCodexAccountStore((s) => s.fetchModels);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<TaskSetupMode>("create");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

  useEffect(() => { fetchCodexModels(); }, [fetchCodexModels]);

  const launchProviders = useCallback(
    async (taskId: string, cwd: string, payload: TaskSetupSubmitPayload) => {
      const claudeAgent = payload.agents.find((a) => a.provider === "claude");
      const codexAgent = payload.agents.find((a) => a.provider === "codex");
      if (claudeAgent) await invoke("daemon_set_claude_role", { role: claudeAgent.role });
      if (codexAgent) await invoke("daemon_set_codex_role", { role: codexAgent.role });
      const cc = claudeAgent ? payload.claudeConfig : null;
      if (cc) {
        const a = cc.historyAction;
        if (a.kind === "resumeNormalized") await resumeSession(a.sessionId);
        else await invoke("daemon_launch_claude_sdk", buildClaudeLaunchRequest({
          claudeRole: claudeAgent!.role, cwd, model: cc.model, effort: cc.effort,
          resumeSessionId: a.kind === "resumeExternal" ? a.externalId : undefined, taskId,
        }));
      }
      const cx = codexAgent ? payload.codexConfig : null;
      if (cx) {
        const a = cx.historyAction;
        if (a.kind === "resumeNormalized") await resumeSession(a.sessionId);
        else await applyConfig(buildCodexLaunchConfig({
          model: cx.model, reasoningEffort: cx.effort, cwd,
          resumeThreadId: a.kind === "resumeExternal" ? a.externalId : undefined, taskId,
        }));
      }
    },
    [applyConfig, resumeSession],
  );

  const handleSetupSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (!selectedWorkspace) return;
      try {
        const newTask = await createTask(selectedWorkspace, "");
        const tid = newTask.taskId;
        for (const def of payload.agents) {
          await addTaskAgent(tid, def.provider, def.role);
        }
        if (payload.requestLaunch) await launchProviders(tid, selectedWorkspace, payload);
      } catch {
        /* task creation or launch error — UI updates via store */
      }
    },
    [addTaskAgent, createTask, launchProviders, selectedWorkspace],
  );

  const handleEditSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (!task) return;
      try {
        const incoming = new Set(payload.agents.filter((d) => d.agentId).map((d) => d.agentId!));
        for (const a of agents) {
          if (!incoming.has(a.agentId)) await removeTaskAgent(a.agentId);
        }
        const finalOrder: string[] = [];
        for (const def of payload.agents) {
          if (def.agentId) {
            await updateTaskAgent(def.agentId, def.provider, def.role, def.displayName);
            finalOrder.push(def.agentId);
          } else {
            const added = await addTaskAgent(task.taskId, def.provider, def.role, def.displayName);
            finalOrder.push(added.agentId);
          }
        }
        if (finalOrder.length > 0) await reorderTaskAgents(task.taskId, finalOrder);
      } catch {
        /* edit error — UI updates via store */
      }
    },
    [addTaskAgent, agents, removeTaskAgent, reorderTaskAgents, task, updateTaskAgent],
  );

  const openDialog = useCallback((m: TaskSetupMode) => {
    setDialogMode(m);
    setDialogOpen(true);
  }, []);

  const requestDeleteTask = useCallback(() => { if (task) setDeleteConfirmOpen(true); }, [task]);
  const handleCancelDelete = useCallback(() => setDeleteConfirmOpen(false), []);
  const handleConfirmDelete = useCallback(async () => {
    if (!task) return;
    setDeleteConfirmOpen(false);
    setDialogOpen(false);
    try { await deleteTask(task.taskId); } catch { /* delete error */ }
  }, [deleteTask, task]);

  const handleDialogSubmit = useCallback(
    (payload: TaskSetupSubmitPayload) => {
      if (dialogMode === "edit") {
        void (async () => {
          try {
            await handleEditSubmit(payload);
            if (payload.requestLaunch && task)
              await launchProviders(task.taskId, task.projectRoot, payload);
          } catch { /* edit/launch error */ }
        })();
      } else {
        void handleSetupSubmit(payload);
      }
      setDialogOpen(false);
    },
    [dialogMode, handleEditSubmit, handleSetupSubmit, launchProviders, task],
  );

  const reviewBadge: ReviewBadge | null =
    task?.status === "reviewing" ? { label: "Review", tone: "warning" } : null;
  const dialogWorkspace = dialogMode === "edit" ? task?.projectRoot : selectedWorkspace;

  return (
    <div className="space-y-2">
      {workspaceTasks.length === 0 && (
        <div className="rounded-xl border border-dashed border-border/50 bg-card/30 px-4 py-3 text-xs text-muted-foreground/70">
          {getTaskPanelEmptyStateMessage()}
        </div>
      )}
      {workspaceTasks.map((t) =>
        t.taskId === activeTaskId ? (
          <TaskHeader
            key={t.taskId}
            task={t}
            reviewBadge={reviewBadge}
            onEditTask={() => openDialog("edit")}
            onDeleteTask={requestDeleteTask}
          />
        ) : (
          <TaskHeader key={t.taskId} task={t} collapsed onClick={() => void selectTask(t.taskId)} />
        ),
      )}
      {selectedWorkspace && !dialogOpen && (
        <button type="button" onClick={() => openDialog("create")}
          className="flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-primary/30 bg-primary/5 px-3 py-2 text-xs font-medium text-primary transition-colors hover:border-primary/50 hover:bg-primary/10">
          <Plus className="size-3.5" /> New Task
        </button>
      )}
      {dialogOpen && dialogWorkspace && (
        <TaskSetupDialog mode={dialogMode} workspace={dialogWorkspace}
          open={dialogOpen} onOpenChange={setDialogOpen} onSubmit={handleDialogSubmit}
          onDelete={dialogMode === "edit" ? requestDeleteTask : undefined}
          initialAgents={dialogMode === "edit" ? agents.map((a) => ({ provider: a.provider, role: a.role, agentId: a.agentId, displayName: a.displayName })) : undefined}
          codexModels={codexModels} />
      )}
      <ConfirmDialog
        open={deleteConfirmOpen}
        title="Delete Task"
        description={`Delete "${task?.title || task?.taskId || "this task"}"? This action cannot be undone.`}
        onConfirm={handleConfirmDelete}
        onCancel={handleCancelDelete}
      />
    </div>
  );
}
