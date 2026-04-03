import { Button } from "@/components/ui/button";
import { FolderOpen, TerminalSquare } from "lucide-react";
import type { ShellMainSurface } from "@/components/shell-layout-state";

interface ShellTopBarProps {
  workspaceLabel: string;
  surfaceMode: ShellMainSurface;
  logLineCount: number;
  errorCount: number;
  onClear: () => void;
}

export function ShellTopBar({
  workspaceLabel,
  surfaceMode,
  logLineCount,
  errorCount,
  onClear,
}: ShellTopBarProps) {
  const hasWorkspace = workspaceLabel !== "No workspace selected";

  return (
    <header className="flex h-14 items-center justify-between border-b border-border/45 px-4">
      <div className="flex items-center gap-3 min-w-0">
        <img
          src="/dimweave-mark.svg"
          alt="Dimweave logo"
          className="h-7 w-7 object-contain shrink-0"
        />
        <span className="text-sm font-semibold text-foreground">
          {surfaceMode === "logs" ? "Runtime logs" : "Dimweave"}
        </span>
        {surfaceMode === "logs" && logLineCount > 0 && (
          <span className="rounded-full border border-border/45 px-2 py-0.5 text-[10px] text-muted-foreground">
            {logLineCount}
          </span>
        )}
        {surfaceMode === "logs" && errorCount > 0 && (
          <span className="inline-flex items-center gap-1 rounded-full border border-destructive/30 bg-destructive/8 px-2 py-0.5 text-[10px] font-medium text-destructive">
            <TerminalSquare className="size-3" />
            {errorCount}
          </span>
        )}
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {surfaceMode === "chat" && (
          <Button variant="secondary" size="xs" onClick={onClear}>
            Clear
          </Button>
        )}
        <div
          className="flex items-center gap-1.5 rounded-full border border-border/45 bg-card/45 px-2.5 py-1 text-[11px] text-muted-foreground"
          title={workspaceLabel}
        >
          <FolderOpen className="size-3 shrink-0 text-muted-foreground/55" />
          {hasWorkspace ? (
            <span className="max-w-[16rem] truncate text-foreground/82">
              {workspaceLabel}
            </span>
          ) : (
            <span className="text-muted-foreground/55">—</span>
          )}
        </div>
      </div>
    </header>
  );
}
