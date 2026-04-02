import {
  AlertTriangle,
  Bot,
  TerminalSquare,
  Workflow,
} from "lucide-react";
import type { ShellNavItem } from "./shell-layout-state";

interface ShellContextBarProps {
  activeItem: ShellNavItem | null;
  onToggle: (item: ShellNavItem) => void;
}

const NAV_ITEMS: Array<{
  id: ShellNavItem;
  label: string;
  icon: typeof Workflow;
}> = [
  { id: "task", label: "Task context", icon: Workflow },
  { id: "agents", label: "Agents", icon: Bot },
  { id: "approvals", label: "Approvals", icon: AlertTriangle },
  { id: "logs", label: "Logs", icon: TerminalSquare },
];

export function ShellContextBar({
  activeItem,
  onToggle,
}: ShellContextBarProps) {
  return (
    <aside className="flex w-20 shrink-0 flex-col border-r border-border/45 bg-background/78 px-3 py-4 backdrop-blur-sm">
      <div className="mb-4 flex h-12 items-center justify-center rounded-2xl border border-border/35 bg-card/55">
        <span className="text-[11px] font-semibold uppercase tracking-[0.24em] text-foreground/88">
          AN
        </span>
      </div>

      <nav className="flex flex-1 flex-col items-center gap-3">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            data-shell-pane-trigger="true"
            aria-label={`Open ${label.toLowerCase()}`}
            aria-pressed={activeItem === id}
            className="group relative flex size-11 items-center justify-center rounded-2xl border border-transparent bg-background/35 text-muted-foreground/72 transition-colors hover:border-border/55 hover:bg-card/80 hover:text-foreground/88 aria-pressed:border-primary/40 aria-pressed:bg-card aria-pressed:text-foreground"
            onClick={() => onToggle(id)}
          >
            <span className="sr-only">{label}</span>
            <Icon className="size-4" />
            {activeItem === id && (
              <span className="absolute -left-3 h-6 w-1 rounded-full bg-primary" />
            )}
          </button>
        ))}
      </nav>
    </aside>
  );
}
