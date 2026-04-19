import { Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buildDraftConfigFromDef } from "@/components/AgentStatus/provider-session-view-model";
import { buildClaudeLaunchRequest } from "@/components/ClaudePanel/launch-request";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useClaudeAccountStore } from "@/stores/claude-account-store";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskAgents,
  selectWorkspaceTasks,
} from "@/stores/task-store/selectors";
import type { Provider, TaskConfig } from "@/stores/task-store/types";
import { TaskHeader, type ReviewBadge } from "./TaskHeader";
import {
  TaskSetupDialog,
  type AgentDef,
  type TaskSetupMode,
  type TaskSetupSubmitPayload,
} from "./TaskSetupDialog";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { getTaskPanelEmptyStateMessage } from "./view-model";

/** Derive lead/coder singleton provider bindings from an agent list. */
export function deriveProviderConfig(agents: AgentDef[]): TaskConfig {
  const lead = agents.find((a) => a.role === "lead");
  const coder = agents.find((a) => a.role === "coder");
  return {
    leadProvider: (lead?.provider ?? "claude") as Provider,
    coderProvider: (coder?.provider ?? "codex") as Provider,
  };
}

export function TaskPanel() {
  const task = useTaskStore(selectActiveTask);
  const workspaceTasks = useTaskStore(selectWorkspaceTasks);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectTask = useTaskStore((s) => s.selectTask);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const selectedWorkspace = useTaskStore((s) => s.selectedWorkspace);
  const agents = useTaskStore(selectActiveTaskAgents);
  const createConfiguredTask = useTaskStore((s) => s.createConfiguredTask);
  const updateTaskConfig = useTaskStore((s) => s.updateTaskConfig);
  const addTaskAgent = useTaskStore((s) => s.addTaskAgent);
  const removeTaskAgent = useTaskStore((s) => s.removeTaskAgent);
  const updateTaskAgent = useTaskStore((s) => s.updateTaskAgent);
  const stopAgent = useTaskStore((s) => s.stopAgent);
  const reorderTaskAgents = useTaskStore((s) => s.reorderTaskAgents);
  const deleteTask = useTaskStore((s) => s.deleteTask);
  const codexModels = useCodexAccountStore((s) => s.models);
  const fetchCodexModels = useCodexAccountStore((s) => s.fetchModels);
  const claudeModels = useClaudeAccountStore((s) => s.models);
  const fetchClaudeModels = useClaudeAccountStore((s) => s.fetchModels);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<TaskSetupMode>("create");
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);

  useEffect(() => {
    fetchCodexModels();
    fetchClaudeModels();
  }, [fetchCodexModels, fetchClaudeModels]);

  const launchProviders = useCallback(
    async (taskId: string, cwd: string, agents: AgentDef[]) => {
      for (const agent of agents) {
        const cfg = buildDraftConfigFromDef(agent);
        const a = cfg.historyAction;
        if (a.kind === "resumeNormalized") {
          await resumeSession(a.sessionId);
          continue;
        }
        if (agent.provider === "claude") {
          await invoke("daemon_set_claude_role", { role: agent.role });
          await invoke(
            "daemon_launch_claude_sdk",
            buildClaudeLaunchRequest({
              claudeRole: agent.role,
              cwd,
              model: cfg.model,
              effort: cfg.effort,
              resumeSessionId:
                a.kind === "resumeExternal" ? a.externalId : undefined,
              taskId,
              agentId: agent.agentId,
            }),
          );
        } else {
          await invoke("daemon_set_codex_role", { role: agent.role });
          await invoke("daemon_launch_codex", {
            roleId: agent.role,
            cwd,
            model: cfg.model || null,
            reasoningEffort: cfg.effort || null,
            resumeThreadId: a.kind === "resumeExternal" ? a.externalId : null,
            taskId,
            agentId: agent.agentId || null,
          });
        }
      }
    },
    [resumeSession],
  );

  const handleSetupSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (!selectedWorkspace) return;
      try {
        const config = deriveProviderConfig(payload.agents);
        const newTask = await createConfiguredTask(
          selectedWorkspace,
          "",
          config,
        );
        const tid = newTask.taskId;
        const savedAgents: AgentDef[] = [];
        for (const def of payload.agents) {
          const added = await addTaskAgent(
            tid,
            def.provider,
            def.role,
            def.displayName,
            def.model,
            def.effort,
          );
          savedAgents.push({ ...def, agentId: added.agentId });
        }
        if (payload.requestLaunch)
          await launchProviders(tid, newTask.taskWorktreeRoot, savedAgents);
      } catch {
        /* task creation or launch error — UI updates via store */
      }
    },
    [addTaskAgent, createConfiguredTask, launchProviders, selectedWorkspace],
  );

  const handleEditSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload): Promise<AgentDef[]> => {
      if (!task) return [];
      try {
        const incoming = new Set(
          payload.agents.filter((d) => d.agentId).map((d) => d.agentId!),
        );
        for (const a of agents) {
          if (!incoming.has(a.agentId)) await removeTaskAgent(a.agentId);
        }
        const savedAgents: AgentDef[] = [];
        const finalOrder: string[] = [];
        for (const def of payload.agents) {
          if (def.agentId) {
            await updateTaskAgent(
              def.agentId,
              def.provider,
              def.role,
              def.displayName,
              def.model,
              def.effort,
            );
            finalOrder.push(def.agentId);
            savedAgents.push(def);
          } else {
            const added = await addTaskAgent(
              task.taskId,
              def.provider,
              def.role,
              def.displayName,
              def.model,
              def.effort,
            );
            finalOrder.push(added.agentId);
            savedAgents.push({ ...def, agentId: added.agentId });
          }
        }
        if (finalOrder.length > 0)
          await reorderTaskAgents(task.taskId, finalOrder);
        const config = deriveProviderConfig(savedAgents);
        await updateTaskConfig(task.taskId, config);
        return savedAgents;
      } catch {
        /* edit error — UI updates via store */
        return [];
      }
    },
    [
      addTaskAgent,
      agents,
      removeTaskAgent,
      reorderTaskAgents,
      task,
      updateTaskAgent,
      updateTaskConfig,
    ],
  );

  const openDialog = useCallback((m: TaskSetupMode) => {
    setDialogMode(m);
    setDialogOpen(true);
  }, []);

  const requestDeleteTask = useCallback(() => {
    if (task) setDeleteConfirmOpen(true);
  }, [task]);
  const handleCancelDelete = useCallback(() => setDeleteConfirmOpen(false), []);
  const handleConfirmDelete = useCallback(async () => {
    if (!task) return;
    setDeleteConfirmOpen(false);
    setDialogOpen(false);
    try {
      await deleteTask(task.taskId);
    } catch {
      /* delete error */
    }
  }, [deleteTask, task]);

  const handleDialogSubmit = useCallback(
    (payload: TaskSetupSubmitPayload) => {
      if (dialogMode === "edit") {
        void (async () => {
          let savedAgents: AgentDef[] = [];
          try {
            savedAgents = await handleEditSubmit(payload);
          } catch (err) {
            console.error("[TaskPanel] edit submit failed:", err);
            return;
          }
          if (!(payload.requestLaunch && task && savedAgents.length > 0)) {
            return;
          }
          // Existing agents may already be online with old config; stop them
          // in parallel so launch re-spawns with the new model/effort
          // (bypasses the daemon's "already online" short-circuit). Use
          // allSettled so one failed stop doesn't abort the whole restart —
          // callers get partial progress instead of a half-dead task.
          const stopAgentIds = savedAgents
            .map((a) => a.agentId)
            .filter((id): id is string => !!id);
          const stopResults = await Promise.allSettled(
            stopAgentIds.map((id) => stopAgent(id)),
          );
          stopResults.forEach((r, i) => {
            if (r.status === "rejected") {
              console.error(
                `[TaskPanel] stopAgent(${stopAgentIds[i]}) failed:`,
                r.reason,
              );
            }
          });
          try {
            await launchProviders(
              task.taskId,
              task.taskWorktreeRoot,
              savedAgents,
            );
          } catch (err) {
            console.error("[TaskPanel] launchProviders failed:", err);
          }
        })();
      } else {
        void handleSetupSubmit(payload);
      }
      setDialogOpen(false);
    },
    [
      dialogMode,
      handleEditSubmit,
      handleSetupSubmit,
      launchProviders,
      stopAgent,
      task,
    ],
  );

  const reviewBadge: ReviewBadge | null =
    task?.status === "reviewing" ? { label: "Review", tone: "warning" } : null;
  const dialogWorkspace =
    dialogMode === "edit" ? task?.projectRoot : selectedWorkspace;

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
          <TaskHeader
            key={t.taskId}
            task={t}
            collapsed
            onClick={() => void selectTask(t.taskId)}
          />
        ),
      )}
      {selectedWorkspace && !dialogOpen && (
        <button
          type="button"
          onClick={() => openDialog("create")}
          className="flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-primary/30 bg-primary/5 px-3 py-2 text-xs font-medium text-primary transition-colors hover:border-primary/50 hover:bg-primary/10"
        >
          <Plus className="size-3.5" /> New Task
        </button>
      )}
      {dialogOpen && dialogWorkspace && (
        <TaskSetupDialog
          mode={dialogMode}
          workspace={dialogWorkspace}
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          onSubmit={handleDialogSubmit}
          onDelete={dialogMode === "edit" ? requestDeleteTask : undefined}
          initialAgents={
            dialogMode === "edit"
              ? agents.map((a) => ({
                  provider: a.provider,
                  role: a.role,
                  agentId: a.agentId,
                  displayName: a.displayName,
                  model: a.model ?? undefined,
                  effort: a.effort ?? undefined,
                }))
              : undefined
          }
          codexModels={codexModels}
          claudeModels={claudeModels}
        />
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
