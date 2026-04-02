import { useEffect, useState } from "react";
import { PanelLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TaskContextPopover } from "@/components/TaskContextPopover";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskArtifactCount,
  selectActiveTaskSessionCount,
} from "@/stores/task-store/selectors";

export function ShellContextBar({
  mobileInspectorOpen = false,
  onToggleMobileInspector,
}: {
  mobileInspectorOpen?: boolean;
  onToggleMobileInspector?: () => void;
}) {
  const [taskContextOpen, setTaskContextOpen] = useState(false);
  const activeTask = useTaskStore(selectActiveTask);
  const sessionCount = useTaskStore(selectActiveTaskSessionCount);
  const artifactCount = useTaskStore(selectActiveTaskArtifactCount);

  useEffect(() => {
    if (mobileInspectorOpen) {
      setTaskContextOpen(false);
    }
  }, [mobileInspectorOpen]);

  return (
    <>
      <div className="pointer-events-none fixed inset-y-0 left-0 z-30">
        <div className="pointer-events-auto absolute left-2 top-1/2 -translate-y-1/2 lg:left-4">
          <button
            type="button"
            data-task-context-trigger="true"
            className="group flex w-14 flex-col items-center gap-2 rounded-2xl border border-border/45 bg-background/88 px-2 py-3 shadow-xl backdrop-blur-sm transition-colors hover:border-border/70 hover:bg-card/92"
            onClick={() => setTaskContextOpen((open) => !open)}
          >
            <PanelLeft className="size-4 text-muted-foreground/78 group-hover:text-foreground/88" />
            <span className="text-[10px] font-medium uppercase tracking-[0.22em] text-muted-foreground/72 [writing-mode:vertical-rl]">
              Task context
            </span>
          </button>
        </div>
      </div>

      {onToggleMobileInspector && (
        <div className="pointer-events-none fixed bottom-24 right-4 z-30 lg:hidden">
          <Button
            size="sm"
            variant={mobileInspectorOpen ? "secondary" : "outline"}
            className="pointer-events-auto shadow-lg backdrop-blur-sm"
            onClick={onToggleMobileInspector}
          >
            {mobileInspectorOpen ? "Close inspector" : "Open inspector"}
          </Button>
        </div>
      )}

      <TaskContextPopover
        open={taskContextOpen}
        onClose={() => setTaskContextOpen(false)}
        task={activeTask}
        sessionCount={sessionCount}
        artifactCount={artifactCount}
      />
    </>
  );
}
