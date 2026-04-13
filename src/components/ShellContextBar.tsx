import { useState, useRef, useEffect } from "react";
import {
  AlertTriangle,
  MessageSquare,
  Moon,
  Sun,
  Monitor,
  RectangleHorizontal,
  SquareRoundCorner,
  TerminalSquare,
  Workflow,
  Wrench,
} from "lucide-react";
import type { RuntimeHealthInfo } from "@/types";
import type { ShellNavItem } from "./shell-layout-state";
import type { ThemeMode } from "./use-theme";
import type { RadiusMode } from "./use-border-radius";

interface ShellContextBarProps {
  activeItem: ShellNavItem | null;
  approvalCount: number;
  bugCount: number;
  messageCount: number;
  runtimeHealth: RuntimeHealthInfo | null;
  themeMode: ThemeMode;
  radiusMode: RadiusMode;
  onToggle: (item: ShellNavItem) => void;
  onThemeChange: (mode: ThemeMode) => void;
  onRadiusToggle: () => void;
}

const NAV_ITEMS: Array<{
  id: ShellNavItem;
  label: string;
  icon: typeof Workflow;
}> = [
  { id: "task", label: "Task context", icon: Workflow },
  { id: "approvals", label: "Approvals", icon: AlertTriangle },
  { id: "bugs", label: "Tools", icon: Wrench },
  { id: "logs", label: "Logs", icon: TerminalSquare },
];

const THEME_OPTIONS: Array<{
  mode: ThemeMode;
  icon: typeof Sun;
  label: string;
}> = [
  { mode: "auto", icon: Monitor, label: "Auto" },
  { mode: "light", icon: Sun, label: "Light" },
  { mode: "dark", icon: Moon, label: "Dark" },
];

export function ShellContextBar({
  activeItem,
  approvalCount,
  bugCount,
  messageCount,
  runtimeHealth,
  themeMode,
  radiusMode,
  onToggle,
  onThemeChange,
  onRadiusToggle,
}: ShellContextBarProps) {
  const [showThemeMenu, setShowThemeMenu] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const ThemeIcon =
    THEME_OPTIONS.find((o) => o.mode === themeMode)?.icon ?? Monitor;

  useEffect(() => {
    if (!showThemeMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node))
        setShowThemeMenu(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showThemeMenu]);
  return (
    <aside className="flex w-14 shrink-0 flex-col items-center border-r border-border/45 bg-background px-2 py-4">
      <nav className="flex flex-1 flex-col items-center gap-2">
        {NAV_ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            data-shell-pane-trigger="true"
            aria-label={`Open ${label.toLowerCase()}`}
            aria-pressed={activeItem === id}
            className="group relative flex size-10 items-center justify-center rounded-xl border border-transparent text-muted-foreground/72 transition-colors hover:border-border/55 hover:bg-card/80 hover:text-foreground/88 aria-pressed:border-primary/40 aria-pressed:bg-card aria-pressed:text-foreground"
            onClick={() => onToggle(id)}
          >
            <span className="sr-only">{label}</span>
            <Icon className="size-4" />
            {id === "approvals" && approvalCount > 0 ? (
              <span className="absolute -right-1 -top-1 min-w-4 rounded-full bg-amber-500 px-1 text-[9px] font-semibold leading-4 text-background">
                {approvalCount > 9 ? "9+" : approvalCount}
              </span>
            ) : null}
            {null}
            {activeItem === id && (
              <span className="absolute -left-2 h-5 w-0.5 rounded-full radius-keep bg-primary" />
            )}
          </button>
        ))}
      </nav>

      <div className="mt-auto flex flex-col items-center gap-3">
        {runtimeHealth ? (
          <button
            type="button"
            aria-label="Runtime degraded"
            title={runtimeHealth.message}
            className={`flex size-10 items-center justify-center rounded-xl transition-colors ${runtimeHealth.level === "error" ? "text-red-500 hover:bg-red-500/10" : "text-amber-500 hover:bg-amber-500/10"}`}
          >
            <AlertTriangle className="size-4" />
            <span className="sr-only">
              Runtime degraded: {runtimeHealth.message}
            </span>
          </button>
        ) : null}
        <button
          type="button"
          onClick={onRadiusToggle}
          aria-label={
            radiusMode === "rounded"
              ? "Switch to sharp corners"
              : "Switch to rounded corners"
          }
          title={radiusMode === "rounded" ? "Sharp" : "Rounded"}
          className="flex size-10 items-center justify-center rounded-xl text-muted-foreground/60 transition-colors hover:bg-card/80 hover:text-foreground/88"
        >
          {radiusMode === "rounded" ? (
            <SquareRoundCorner className="size-4" />
          ) : (
            <RectangleHorizontal className="size-4" />
          )}
        </button>
        <div className="relative" ref={menuRef}>
          <button
            type="button"
            onClick={() => setShowThemeMenu((v) => !v)}
            aria-label="Theme"
            className="flex size-10 items-center justify-center rounded-xl text-muted-foreground/60 transition-colors hover:bg-card/80 hover:text-foreground/88"
          >
            <ThemeIcon className="size-4" />
          </button>
          {showThemeMenu && (
            <div className="absolute left-full top-1/2 -translate-y-1/2 z-50 ml-2 w-28 rounded-xl border border-border bg-popover py-1 shadow-xl animate-in fade-in zoom-in-95 duration-150">
              {THEME_OPTIONS.map(({ mode, icon: Icon, label }) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => {
                    onThemeChange(mode);
                    setShowThemeMenu(false);
                  }}
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-[11px] transition-colors hover:bg-accent ${mode === themeMode ? "text-primary font-semibold" : "text-foreground"}`}
                >
                  <Icon className="size-3.5" />
                  {label}
                </button>
              ))}
            </div>
          )}
        </div>
        <div className="flex flex-col items-center gap-1 text-muted-foreground/50">
          <MessageSquare className="size-3.5" />
          <span className="text-[9px] tabular-nums">{messageCount}</span>
        </div>
      </div>
    </aside>
  );
}
