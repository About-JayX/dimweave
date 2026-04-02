import { useEffect, useRef } from "react";
import { FolderTree, TerminalSquare, Workflow, X } from "lucide-react";
import { shortenPath } from "@/lib/utils";
import { Button } from "./ui/button";
import { ReviewGateBadge } from "./TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "./TaskPanel/view-model";
import type { TaskInfo } from "@/stores/task-store/types";

interface TaskContextPopoverProps {
  open: boolean;
  onClose: () => void;
  task: TaskInfo | null;
  sessionCount: number;
  artifactCount: number;
}

export function TaskContextPopover({
  open,
  onClose,
  task,
  sessionCount,
  artifactCount,
}: TaskContextPopoverProps) {
  const panelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target) return;

      if (panelRef.current?.contains(target)) {
        return;
      }

      const trigger = (target as HTMLElement).closest?.(
        "[data-task-context-trigger='true']",
      );
      if (trigger) {
        return;
      }

      onClose();
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("pointerdown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("pointerdown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose, open]);

  if (!open) return null;

  const reviewBadge = getReviewBadge(task?.reviewStatus);

  return (
    <div className="pointer-events-none fixed left-20 top-4 z-40 max-lg:left-4 max-lg:top-16">
      <div
        ref={panelRef}
        data-task-context-drawer="true"
        className="pointer-events-auto h-[calc(100vh-2rem)] w-[min(24rem,calc(100vw-2rem))] overflow-hidden rounded-2xl border border-border/45 bg-background/96 shadow-2xl backdrop-blur-sm animate-in slide-in-from-left-2 duration-200 max-lg:h-[calc(100vh-5rem)]"
      >
        <div className="flex items-start justify-between border-b border-border/35 px-4 py-3">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              Task context
            </div>
            <div className="mt-0.5 text-sm font-semibold text-foreground">
              {task ? task.title : "No active task"}
            </div>
          </div>
          <Button size="xs" variant="ghost" onClick={onClose}>
            <X className="size-3.5" />
            Close
          </Button>
        </div>

        <div className="space-y-3 px-4 py-4 text-[12px] text-muted-foreground/78">
          {task ? (
            <>
              <div className="inline-flex items-center gap-2 rounded-full border border-border/40 bg-card/60 px-3 py-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground/70">
                <FolderTree className="size-3.5" />
                {shortenPath(task.workspaceRoot)}
              </div>
              <div className="grid gap-2 sm:grid-cols-2">
                <div className="rounded-xl border border-border/35 bg-card/55 px-3 py-2">
                  <div className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground/55">
                    Sessions
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-sm text-foreground">
                    <Workflow className="size-3.5 text-muted-foreground/70" />
                    {sessionCount} sessions
                  </div>
                </div>
                <div className="rounded-xl border border-border/35 bg-card/55 px-3 py-2">
                  <div className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground/55">
                    Artifacts
                  </div>
                  <div className="mt-1 flex items-center gap-2 text-sm text-foreground">
                    <TerminalSquare className="size-3.5 text-muted-foreground/70" />
                    {artifactCount} artifacts
                  </div>
                </div>
              </div>
              {reviewBadge && (
                <div className="pt-1">
                  <ReviewGateBadge badge={reviewBadge} />
                </div>
              )}
            </>
          ) : (
            <div className="rounded-xl border border-dashed border-border/40 bg-card/35 px-3 py-3 text-sm text-muted-foreground/72">
              No active task.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
