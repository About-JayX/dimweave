import { Plus } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buildCodexLaunchConfig } from "@/components/AgentStatus/codex-launch-config";
import { buildClaudeLaunchRequest } from "@/components/ClaudePanel/launch-request";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskArtifacts,
  selectActiveTaskSessions,
} from "@/stores/task-store/selectors";
import { ArtifactTimeline } from "./ArtifactTimeline";
import { SessionTree } from "./SessionTree";
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
  const createConfiguredTask = useTaskStore((s) => s.createConfiguredTask);
  const updateTaskConfig = useTaskStore((s) => s.updateTaskConfig);
  const claudeRole = useBridgeStore((s) => s.claudeRole);
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

  const handleResume = useCallback(
    (sessionId: string) => {
      void resumeSession(sessionId);
    },
    [resumeSession],
  );

  const handleSetupSubmit = useCallback(
    async (payload: TaskSetupSubmitPayload) => {
      if (dialogMode === "create" && selectedWorkspace) {
        try {
          const newTask = await createConfiguredTask(selectedWorkspace, "", {
            leadProvider: payload.leadProvider,
            coderProvider: payload.coderProvider,
          });
          if (payload.requestLaunch) {
            const tid = newTask.taskId;
            const cwd = selectedWorkspace;
            const wantsClaude = payload.leadProvider === "claude" || payload.coderProvider === "claude";
            const wantsCodex = payload.leadProvider === "codex" || payload.coderProvider === "codex";
            const cc = wantsClaude ? payload.claudeConfig : null;
            if (cc) {
              const a = cc.historyAction;
              if (a.kind === "resumeNormalized") await resumeSession(a.sessionId);
              else await invoke("daemon_launch_claude_sdk", buildClaudeLaunchRequest({
                claudeRole, cwd, model: cc.model, effort: cc.effort,
                resumeSessionId: a.kind === "resumeExternal" ? a.externalId : undefined,
                taskId: tid,
              }));
            }
            const cx = wantsCodex ? payload.codexConfig : null;
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
      } else if (dialogMode === "edit" && task) {
        void updateTaskConfig(task.taskId, {
          leadProvider: payload.leadProvider,
          coderProvider: payload.coderProvider,
        });
      }
    },
    [
      applyConfig,
      claudeRole,
      createConfiguredTask,
      dialogMode,
      resumeSession,
      selectedWorkspace,
      task,
      updateTaskConfig,
    ],
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
            onClick={() => {
              setDialogMode("create");
              setDialogOpen(true);
            }}
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
            onSubmit={handleSetupSubmit}
          />
        )}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <TaskHeader
        task={task}
        reviewBadge={reviewBadge}
        onEditTask={() => {
          setDialogMode("edit");
          setDialogOpen(true);
        }}
      />
      {dialogOpen && (
        <TaskSetupDialog
          mode="edit"
          workspace={task.workspaceRoot}
          open={dialogOpen}
          onOpenChange={setDialogOpen}
          onSubmit={handleSetupSubmit}
          initialLeadProvider={task.leadProvider}
          initialCoderProvider={task.coderProvider}
        />
      )}
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
    </div>
  );
}
