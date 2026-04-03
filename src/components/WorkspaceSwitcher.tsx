import { useEffect, useRef, useState } from "react";
import { ChevronDown, FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { shortenPath } from "@/lib/utils";
import type { WorkspaceCandidate } from "./workspace-entry-state";

interface WorkspaceSwitcherProps {
  workspaceLabel: string;
  currentWorkspace: string | null;
  selected: WorkspaceCandidate | null;
  recentWorkspaces: string[];
  actionError: string | null;
  onChooseFolder: () => void;
  onSelectRecent: (workspace: WorkspaceCandidate) => void;
  onContinue: () => void;
  defaultOpen?: boolean;
}

export function WorkspaceSwitcher({
  workspaceLabel,
  currentWorkspace,
  selected,
  recentWorkspaces,
  actionError,
  onChooseFolder,
  onSelectRecent,
  onContinue,
  defaultOpen = false,
}: WorkspaceSwitcherProps) {
  const [open, setOpen] = useState(defaultOpen);
  const previousSelectedPathRef = useRef<string | null>(selected?.path ?? null);
  const selectedPath = selected?.path ?? null;
  const buttonLabel = currentWorkspace ? workspaceLabel : "Choose workspace";

  useEffect(() => {
    const previousSelectedPath = previousSelectedPathRef.current;
    if (open && previousSelectedPath && !selectedPath && !actionError) {
      setOpen(false);
    }
    previousSelectedPathRef.current = selectedPath;
  }, [actionError, open, selectedPath]);

  return (
    <div className="relative">
      <Button
        type="button"
        variant="outline"
        size="xs"
        aria-expanded={open}
        className="max-w-[18rem] gap-1.5 rounded-full border-border/45 bg-card/45 pl-2 pr-2.5"
        title={currentWorkspace ?? workspaceLabel}
        onClick={() => setOpen((current) => !current)}
      >
        <FolderOpen className="size-3 shrink-0 text-muted-foreground/55" />
        <span className="truncate text-foreground/82">{buttonLabel}</span>
        <ChevronDown className="size-3 shrink-0 text-muted-foreground/55" />
      </Button>

      {open && (
        <div className="absolute right-0 top-full z-50 mt-2 w-[26rem] rounded-2xl border border-border/60 bg-popover/95 p-3 shadow-xl backdrop-blur-sm animate-in fade-in zoom-in-95 duration-150">
          <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground/70">
            {currentWorkspace ? "Switch workspace" : "Choose workspace"}
          </div>

          <Button
            variant="outline"
            size="sm"
            className="mt-3 w-full justify-center"
            onClick={onChooseFolder}
          >
            Choose folder...
          </Button>

          {selected?.type === "picked" && (
            <div
              data-workspace-selected="true"
              className="mt-3 rounded-2xl border border-primary/35 bg-primary/8 px-4 py-3"
            >
              <div className="text-xs uppercase tracking-[0.18em] text-primary/70">
                Selected workspace
              </div>
              <div className="mt-1 font-mono text-sm text-foreground">
                {selected.path}
              </div>
            </div>
          )}

          {recentWorkspaces.length > 0 && (
            <div className="mt-3 space-y-2">
              <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground/70">
                Recent workspaces
              </div>
              <div className="space-y-2">
                {recentWorkspaces.map((workspace) => {
                  const isCurrent = currentWorkspace === workspace;
                  const isSelected = selectedPath === workspace;

                  return (
                    <button
                      key={workspace}
                      type="button"
                      data-workspace-selected={isSelected || undefined}
                      className="flex w-full items-center justify-between rounded-2xl border border-border/50 bg-background/60 px-4 py-3 text-left transition-colors hover:border-border"
                      onClick={() =>
                        onSelectRecent({ type: "recent", path: workspace })
                      }
                    >
                      <span className="font-mono text-sm text-foreground">
                        {workspace}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {isCurrent ? "Current" : shortenPath(workspace)}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {actionError && (
            <div className="mt-3 rounded-2xl border border-destructive/25 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {actionError}
            </div>
          )}

          <Button
            size="sm"
            className="mt-3 w-full"
            disabled={!selected}
            onClick={onContinue}
          >
            Continue
          </Button>
        </div>
      )}
    </div>
  );
}
