import { Plus } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buildCodexLaunchConfig } from "@/components/AgentStatus/codex-launch-config";
import { buildClaudeLaunchRequest } from "@/components/ClaudePanel/launch-request";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskAgents,
  selectActiveTaskArtifacts,
  selectActiveTaskSessions,
} from "@/stores/task-store/selectors";
import { ArtifactTimeline } from "./ArtifactTimeline";
import { SessionTree } from "./SessionTree";
import { TaskAgentList } from "./TaskAgentList";
import { TaskHeader, type ReviewBadge } from "./TaskHeader";
import {
  TaskSetupDialog,
  type TaskSetupMode,
  type TaskSetupSubmitPayload,
} from "./TaskSetupDialog";
import { useArtifactDetail } from "./use-artifact-detail";
import {
  buildArtifactTimeline,
  buildSessionTreeRows,
  getTaskPanelEmptyStateMessage,
} from "./view-model";

export function TaskPanel() {
  const task = useTaskStore(selectActiveTask);
  const taskSessions = useTaskStore(selectActiveTaskSessions);
  const taskArtifacts = useTaskStore(selectActiveTaskArtifacts);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const selectedWorkspace = useTaskStore((s) => s.selectedWorkspace);
  const agents = useTaskStore(selectActiveTaskAgents);
  const createTask = useTaskStore((s) => s.createTask);
  const addTaskAgent = useTaskStore((s) => s.addTaskAgent);
  const removeTaskAgent = useTaskStore((s) => s.removeTaskAgent);
  const updateTaskAgent = useTaskStore((s) => s.updateTaskAgent);
  const applyConfig = useBridgeStore((s) => s.applyConfig);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<TaskSetupMode>("create");
  const sessionRows = useMemo(
    () => buildSessionTreeRows(taskSessions, task),
    [task, taskSessions],
  );
  const artifactTimeline = useMemo(
    () => buildArtifactTimeline(taskArtifacts, taskSessions),
    [taskArtifacts, taskSessions],
  );
  const {
    selectedArtifactId,
    setSelectedArtifactId,
    detail: artifactDetailModel,
    detailLoading: artifactDetailLoading,
    detailError: artifactDetailError,
  } = useArtifactDetail(artifactTimeline);

  const handleResume = useCallback((id: string) => void resumeSession(id), [resumeSession]);

  const handleSetupSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (!selectedWorkspace) return;
      try {
        const newTask = await createTask(selectedWorkspace, "");
        const tid = newTask.taskId;
        for (const def of payload.agents) {
          await addTaskAgent(tid, def.provider, def.role);
        }
        if (payload.requestLaunch) {
          const cwd = selectedWorkspace;
          const claudeAgent = payload.agents.find((a) => a.provider === "claude");
          const codexAgent = payload.agents.find((a) => a.provider === "codex");
          if (claudeAgent) {
            await invoke("daemon_set_claude_role", { role: claudeAgent.role });
          }
          if (codexAgent) {
            await invoke("daemon_set_codex_role", { role: codexAgent.role });
          }
          const cc = claudeAgent ? payload.claudeConfig : null;
          if (cc) {
            const a = cc.historyAction;
            if (a.kind === "resumeNormalized") await resumeSession(a.sessionId);
            else await invoke("daemon_launch_claude_sdk", buildClaudeLaunchRequest({
              claudeRole: claudeAgent!.role, cwd, model: cc.model, effort: cc.effort,
              resumeSessionId: a.kind === "resumeExternal" ? a.externalId : undefined,
              taskId: tid,
            }));
          }
          const cx = codexAgent ? payload.codexConfig : null;
          if (cx) {
            const a = cx.historyAction;
            if (a.kind === "resumeNormalized") await resumeSession(a.sessionId);
            else await applyConfig(buildCodexLaunchConfig({
              model: cx.model, reasoningEffort: cx.effort, cwd,
              resumeThreadId: a.kind === "resumeExternal" ? a.externalId : undefined,
              taskId: tid,
            }));
          }
        }
      } catch {
        /* task creation or launch error — UI updates via store */
      }
    },
    [addTaskAgent, applyConfig, createTask, resumeSession, selectedWorkspace],
  );

  const handleEditSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (!task) return;
      try {
        const incoming = new Set(payload.agents.filter((d) => d.agentId).map((d) => d.agentId!));
        for (const a of agents) {
          if (!incoming.has(a.agentId)) await removeTaskAgent(a.agentId);
        }
        for (const def of payload.agents) {
          if (def.agentId) {
            await updateTaskAgent(def.agentId, def.provider, def.role, def.displayName);
          } else {
            await addTaskAgent(task.taskId, def.provider, def.role, def.displayName);
          }
        }
      } catch {
        /* edit error — UI updates via store */
      }
    },
    [addTaskAgent, agents, removeTaskAgent, task, updateTaskAgent],
  );

  const openDialog = useCallback((m: TaskSetupMode) => {
    setDialogMode(m);
    setDialogOpen(true);
  }, []);

  const handleDialogSubmit = useCallback(
    (payload: TaskSetupSubmitPayload) => {
      void (dialogMode === "edit" ? handleEditSubmit(payload) : handleSetupSubmit(payload));
      setDialogOpen(false);
    },
    [dialogMode, handleEditSubmit, handleSetupSubmit],
  );

  const reviewBadge: ReviewBadge | null =
    task?.status === "reviewing" ? { label: "Review", tone: "warning" } : null;

  if (!task) {
    return (
      <div className="space-y-3">
        <div className="rounded-xl border border-dashed border-border/50 bg-card/30 px-4 py-3 text-xs text-muted-foreground/70">
          {getTaskPanelEmptyStateMessage()}
        </div>
        {selectedWorkspace && !dialogOpen && (
          <button
            type="button"
            onClick={() => openDialog("create")}
            className="flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-primary/30 bg-primary/5 px-3 py-2 text-xs font-medium text-primary transition-colors hover:border-primary/50 hover:bg-primary/10"
          >
            <Plus className="size-3.5" />
            New Task
          </button>
        )}
        {dialogOpen && selectedWorkspace && (
          <TaskSetupDialog
            mode="create"
            workspace={selectedWorkspace}
            open={dialogOpen}
            onOpenChange={setDialogOpen}
            onSubmit={handleDialogSubmit}
          />
        )}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <TaskHeader task={task} reviewBadge={reviewBadge} onEditTask={() => openDialog("edit")} />
      <TaskAgentList />
      <div className="rounded-2xl border border-border/50 bg-card/50 p-0">
        <SessionTree rows={sessionRows} onResume={handleResume} />
      </div>
      <div className="rounded-2xl border border-border/50 bg-card/50 p-0">
        <ArtifactTimeline
          items={artifactTimeline}
          selectedArtifactId={selectedArtifactId}
          detail={artifactDetailModel}
          detailLoading={artifactDetailLoading}
          detailError={artifactDetailError}
          onSelect={setSelectedArtifactId}
        />
      </div>
      {dialogOpen && task.workspaceRoot && (
        <TaskSetupDialog mode={dialogMode} workspace={task.workspaceRoot}
          open={dialogOpen} onOpenChange={setDialogOpen} onSubmit={handleDialogSubmit}
          initialAgents={dialogMode === "edit" ? agents.map((a) => ({ provider: a.provider, role: a.role, agentId: a.agentId, displayName: a.displayName })) : undefined} />
      )}
    </div>
  );
}
