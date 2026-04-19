import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import type {
  PermissionBehavior,
  PermissionPrompt,
  PermissionResolutionError,
} from "@/types";

interface PermissionQueueProps {
  prompts: PermissionPrompt[];
  onResolve: (requestId: string, behavior: PermissionBehavior) => Promise<void>;
}

interface PermissionQueueViewProps extends PermissionQueueProps {
  error: PermissionResolutionError | null;
}

export function PermissionQueue({ prompts, onResolve }: PermissionQueueProps) {
  const permissionError = useBridgeStore((s) => s.permissionError);
  return (
    <PermissionQueueView
      prompts={prompts}
      error={permissionError}
      onResolve={onResolve}
    />
  );
}

export function PermissionQueueView({
  prompts,
  error,
  onResolve,
}: PermissionQueueViewProps) {
  const [busyId, setBusyId] = useState<string | null>(null);
  const tasks = useTaskStore((s) => s.tasks);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);

  const taskLabel = (taskId: string | undefined): string | null => {
    if (!taskId) return null;
    const task = tasks[taskId];
    return task?.title?.trim() || task?.taskId || taskId;
  };

  const handleResolve = async (
    requestId: string,
    behavior: PermissionBehavior,
  ) => {
    setBusyId(requestId);
    try {
      await onResolve(requestId, behavior);
    } finally {
      setBusyId((current) => (current === requestId ? null : current));
    }
  };

  if (prompts.length === 0) {
    return (
      <div className="py-10 text-center text-[13px] text-muted-foreground">
        No pending approvals.
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
      {error ? (
        <div className="rounded-xl border border-destructive/35 bg-destructive/8 px-3 py-2 text-[12px] text-destructive">
          <div className="font-medium">Last action failed</div>
          <div className="mt-1 text-[11px] text-destructive/85">
            {error.message}
          </div>
        </div>
      ) : null}
      {prompts.map((prompt) => {
        const busy = busyId === prompt.requestId;
        const label = taskLabel(prompt.taskId);
        const isOtherTask =
          !!prompt.taskId &&
          activeTaskId !== null &&
          prompt.taskId !== activeTaskId;
        return (
          <div
            key={prompt.requestId}
            className="rounded-xl border border-amber-500/30 bg-amber-500/8 p-3"
          >
            <div className="flex items-center justify-between gap-3">
              <div>
                <div className="flex items-center gap-1.5">
                  <span className="text-[13px] font-medium text-foreground">
                    {prompt.toolName}
                  </span>
                  {label && (
                    <span
                      className={
                        isOtherTask
                          ? "rounded-full bg-amber-500/20 px-1.5 py-px text-[9px] font-semibold uppercase text-amber-600"
                          : "rounded-full bg-primary/10 px-1.5 py-px text-[9px] font-semibold uppercase text-primary"
                      }
                      title={
                        isOtherTask
                          ? `Pending on another task: ${label}`
                          : undefined
                      }
                    >
                      {label}
                    </span>
                  )}
                </div>
                <div className="mt-0.5 text-[11px] text-muted-foreground">
                  {prompt.agent} •{" "}
                  {new Date(prompt.createdAt).toLocaleTimeString()}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  size="xs"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => handleResolve(prompt.requestId, "deny")}
                >
                  Deny
                </Button>
                <Button
                  size="xs"
                  disabled={busy}
                  onClick={() => handleResolve(prompt.requestId, "allow")}
                >
                  Allow
                </Button>
              </div>
            </div>
            <p className="mt-3 text-[12px] leading-relaxed text-secondary-foreground">
              {prompt.description}
            </p>
            {prompt.inputPreview && (
              <pre className="mt-3 overflow-x-auto rounded-lg border border-border/60 bg-background/70 p-2 font-mono text-[11px] text-muted-foreground">
                {prompt.inputPreview}
              </pre>
            )}
          </div>
        );
      })}
    </div>
  );
}
