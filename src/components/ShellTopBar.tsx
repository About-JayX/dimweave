import { Search, TerminalSquare } from "lucide-react";
import type { ShellMainSurface } from "@/components/shell-layout-state";
import { Button } from "@/components/ui/button";
import { WorkspaceSwitcher } from "@/components/WorkspaceSwitcher";
import type { WorkspaceCandidate } from "@/components/workspace-entry-state";

interface ShellTopBarProps {
  workspaceLabel: string;
  currentWorkspace: string | null;
  selectedWorkspace: WorkspaceCandidate | null;
  recentWorkspaces: string[];
  workspaceActionError: string | null;
  surfaceMode: ShellMainSurface;
  logLineCount: number;
  errorCount: number;
  onClear: () => void;
  onSearchToggle?: () => void;
  onChooseWorkspace: () => void;
  onSelectRecentWorkspace: (workspace: WorkspaceCandidate) => void;
  onContinueIntoWorkspace: () => void;
}

export function ShellTopBar({
  workspaceLabel,
  currentWorkspace,
  selectedWorkspace,
  recentWorkspaces,
  workspaceActionError,
  surfaceMode,
  logLineCount,
  errorCount,
  onClear,
  onSearchToggle,
  onChooseWorkspace,
  onSelectRecentWorkspace,
  onContinueIntoWorkspace,
}: ShellTopBarProps) {
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
        {surfaceMode === "chat" && onSearchToggle && (
          <button
            type="button"
            onClick={onSearchToggle}
            className="rounded-md p-1 text-muted-foreground/50 hover:text-foreground transition-colors"
            aria-label="Search messages"
          >
            <Search className="size-4" />
          </button>
        )}
        {surfaceMode === "chat" && (
          <Button variant="secondary" size="xs" onClick={onClear}>
            Clear
          </Button>
        )}
        <WorkspaceSwitcher
          workspaceLabel={workspaceLabel}
          currentWorkspace={currentWorkspace}
          selected={selectedWorkspace}
          recentWorkspaces={recentWorkspaces}
          actionError={workspaceActionError}
          onChooseFolder={onChooseWorkspace}
          onSelectRecent={onSelectRecentWorkspace}
          onContinue={onContinueIntoWorkspace}
        />
      </div>
    </header>
  );
}
