import { useEffect, useRef } from "react";
import { AlertTriangle, Bot, Workflow, X } from "lucide-react";
import { Button } from "./ui/button";
import { AgentStatusPanel } from "./AgentStatus";
import { TaskPanel } from "./TaskPanel";
import { PermissionQueue } from "./MessagePanel/PermissionQueue";
import { useBridgeStore } from "@/stores/bridge-store";
import type { TaskInfo } from "@/stores/task-store/types";
import type { ShellSidebarPane } from "./shell-layout-state";

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
  const panelRef = useRef<HTMLDivElement | null>(null);
  const permissionPrompts = useBridgeStore((s) => s.permissionPrompts);
  const respondToPermission = useBridgeStore((s) => s.respondToPermission);

  useEffect(() => {
    if (!activePane) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target) return;

      if (panelRef.current?.contains(target)) {
        return;
      }

      const trigger = (target as HTMLElement).closest?.(
        "[data-shell-pane-trigger='true']",
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
  }, [activePane, onClose]);

  if (!activePane) return null;

  const paneMeta =
    activePane === "task"
      ? {
          eyebrow: "Task context",
          title: task?.title ?? "Task workspace",
          icon: Workflow,
          content: <TaskPanel />,
        }
      : activePane === "agents"
        ? {
            eyebrow: "Agents",
            title: "Runtime control",
            icon: Bot,
            content: <AgentStatusPanel />,
          }
        : {
            eyebrow: "Approvals",
            title: "Permission queue",
            icon: AlertTriangle,
            content: (
              <PermissionQueue
                prompts={permissionPrompts}
                onResolve={respondToPermission}
              />
            ),
          };
  const Icon = paneMeta.icon;

  return (
    <div className="pointer-events-none fixed left-20 top-4 z-40 max-lg:left-16 max-lg:top-4">
      <div
        ref={panelRef}
        data-shell-sidebar-drawer="true"
        className="pointer-events-auto flex h-[calc(100vh-2rem)] w-[min(26rem,calc(100vw-6rem))] min-w-[22rem] flex-col overflow-hidden rounded-2xl border border-border/45 bg-background/96 shadow-2xl backdrop-blur-sm animate-in slide-in-from-left-2 duration-200 max-lg:min-w-0 max-lg:w-[min(24rem,calc(100vw-5rem))]"
      >
        <div className="flex items-start justify-between border-b border-border/35 px-4 py-3">
          <div className="flex items-start gap-3">
            <div className="mt-0.5 rounded-xl border border-border/35 bg-background/55 p-2 text-muted-foreground/72">
              <Icon className="size-4" />
            </div>
            <div>
              <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
                {paneMeta.eyebrow}
              </div>
              <div className="mt-0.5 text-sm font-semibold text-foreground">
                {paneMeta.title}
              </div>
            </div>
          </div>
          <Button size="xs" variant="ghost" onClick={onClose}>
            <X className="size-3.5" />
            Close
          </Button>
        </div>

        <div className="min-h-0 overflow-y-auto px-4 py-4 text-[12px] text-muted-foreground/78">
          {paneMeta.content}
        </div>
      </div>
    </div>
  );
}
