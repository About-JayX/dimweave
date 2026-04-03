import { useCallback, useEffect, useRef, useState } from "react";
import { AlertTriangle, Bot, Workflow } from "lucide-react";
import { AgentStatusPanel } from "./AgentStatus";
import { TaskPanel } from "./TaskPanel";
import { PermissionQueue } from "./MessagePanel/PermissionQueue";
import { useBridgeStore } from "@/stores/bridge-store";
import { cn } from "@/lib/utils";
import type { TaskInfo } from "@/stores/task-store/types";
import {
  getMountedShellPanes,
  type ShellSidebarPane,
} from "./shell-layout-state";

const STORAGE_KEY = "dimweave:sidebar-width";
const MIN_WIDTH = 280;
const MAX_WIDTH = 640;
const DEFAULT_WIDTH = MIN_WIDTH;

export function normalizeSidebarWidth(value: string | null): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return DEFAULT_WIDTH;
  }
  return Math.min(Math.max(parsed, MIN_WIDTH), MAX_WIDTH);
}

function loadSidebarWidth(): number {
  try {
    return normalizeSidebarWidth(localStorage.getItem(STORAGE_KEY));
  } catch {}
  return DEFAULT_WIDTH;
}

interface TaskContextPopoverProps {
  activePane: ShellSidebarPane | null;
  onClose: () => void;
  task: TaskInfo | null;
}

export function TaskContextPopover({
  activePane,
  onClose,
  task,
}: TaskContextPopoverProps) {
  const permissionPrompts = useBridgeStore((s) => s.permissionPrompts);
  const respondToPermission = useBridgeStore((s) => s.respondToPermission);
  const [mountedPanes, setMountedPanes] = useState<ShellSidebarPane[]>(() =>
    getMountedShellPanes([], activePane),
  );
  const [width, setWidth] = useState(loadSidebarWidth);
  const outerRef = useRef<HTMLDivElement>(null);
  const innerRef = useRef<HTMLDivElement>(null);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      const startX = e.clientX;
      const startW = width;
      const outer = outerRef.current;
      const inner = innerRef.current;
      if (outer) outer.style.transition = "none";

      let rafId = 0;
      let latestW = startW;

      const applyWidth = () => {
        if (outer) outer.style.width = `${latestW}px`;
        if (inner) {
          inner.style.width = `${latestW}px`;
          inner.style.minWidth = `${latestW}px`;
        }
        rafId = 0;
      };

      const onMove = (ev: PointerEvent) => {
        latestW = Math.min(
          Math.max(startW + ev.clientX - startX, MIN_WIDTH),
          MAX_WIDTH,
        );
        if (!rafId) rafId = requestAnimationFrame(applyWidth);
      };
      const onUp = (ev: PointerEvent) => {
        if (rafId) cancelAnimationFrame(rafId);
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
        const finalW = Math.min(
          Math.max(startW + ev.clientX - startX, MIN_WIDTH),
          MAX_WIDTH,
        );
        if (outer) outer.style.transition = "";
        setWidth(finalW);
        try {
          localStorage.setItem(STORAGE_KEY, String(finalW));
        } catch {}
      };
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
    },
    [width],
  );

  useEffect(() => {
    setMountedPanes((current) => getMountedShellPanes(current, activePane));
  }, [activePane]);

  useEffect(() => {
    if (!activePane) return;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [activePane, onClose]);
  const paneMeta = {
    task: {
      eyebrow: "Task context",
      title: task?.title ?? "Task workspace",
      icon: Workflow,
    },
    agents: {
      eyebrow: "Agents",
      title: "Runtime control",
      icon: Bot,
    },
    approvals: {
      eyebrow: "Approvals",
      title: "Permission queue",
      icon: AlertTriangle,
    },
  } satisfies Record<
    ShellSidebarPane,
    { eyebrow: string; title: string; icon: typeof Workflow }
  >;
  const activeMeta = activePane ? paneMeta[activePane] : paneMeta.task;
  const ActiveIcon = activeMeta.icon;

  return (
    <div
      ref={outerRef}
      data-shell-sidebar-panel="true"
      className={cn(
        "relative min-h-0 shrink-0 overflow-hidden border-r border-border/45 bg-background transition-[width,opacity] duration-200",
        !activePane && "w-0 opacity-0",
      )}
      style={activePane ? { width: `${width}px`, opacity: 1 } : undefined}
    >
      <div
        ref={innerRef}
        className={cn(
          "flex h-full flex-col",
          activePane ? "pointer-events-auto" : "pointer-events-none",
        )}
        style={{ width: `${width}px`, minWidth: `${width}px` }}
      >
        {/* Resize handle */}
        <div
          onPointerDown={onPointerDown}
          className="absolute inset-y-0 right-0 z-10 w-1 cursor-col-resize hover:bg-primary/25 active:bg-primary/40 transition-colors"
        />
        <div className="flex items-center gap-3 border-b border-border/35 px-4 py-3">
          <div className="rounded-xl border border-border/35 bg-background/55 p-2 text-muted-foreground/72">
            <ActiveIcon className="size-4" />
          </div>
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              {activeMeta.eyebrow}
            </div>
            <div className="mt-0.5 text-sm font-semibold text-foreground">
              {activeMeta.title}
            </div>
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-hidden">
          {mountedPanes.includes("task") && (
            <div
              className={cn(
                "h-full overflow-y-auto px-4 py-4 text-[12px] text-muted-foreground/78",
                activePane === "task" ? "block" : "hidden",
              )}
            >
              <TaskPanel />
            </div>
          )}

          {mountedPanes.includes("agents") && (
            <div
              className={cn(
                "h-full overflow-y-auto px-4 py-4 text-[12px] text-muted-foreground/78",
                activePane === "agents" ? "block" : "hidden",
              )}
            >
              <AgentStatusPanel />
            </div>
          )}

          {mountedPanes.includes("approvals") && (
            <div
              className={cn(
                "h-full overflow-y-auto px-4 py-4 text-[12px] text-muted-foreground/78",
                activePane === "approvals" ? "block" : "hidden",
              )}
            >
              <PermissionQueue
                prompts={permissionPrompts}
                onResolve={respondToPermission}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
