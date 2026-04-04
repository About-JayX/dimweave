import { Button } from "@/components/ui/button";
import { shortenPath } from "@/lib/utils";
import type { WorkspaceCandidate } from "./workspace-entry-state";

interface WorkspaceEntryOverlayProps {
  selected: WorkspaceCandidate | null;
  recentWorkspaces: string[];
  actionError: string | null;
  onChooseFolder: () => void;
  onSelectRecent: (workspace: WorkspaceCandidate) => void;
  onContinue: () => void;
}

export function WorkspaceEntryOverlay({
  selected,
  recentWorkspaces,
  actionError,
  onChooseFolder,
  onSelectRecent,
  onContinue,
}: WorkspaceEntryOverlayProps) {
  const selectedPath = selected?.path ?? null;

  return (
    <div className="absolute inset-0 z-40 flex items-center justify-center bg-background/90 px-6 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded-[2rem] border border-border/50 bg-card/95 p-8 shadow-2xl">
        <img
          src="/dimweave-mark.svg"
          alt="Dimweave logo"
          className="mx-auto h-14 w-14 object-contain"
        />
        <div className="mt-5 text-center text-[11px] uppercase tracking-[0.24em] text-muted-foreground">
          Dimweave
        </div>
        <h1 className="mt-5 text-center text-3xl font-semibold">
          Choose your workspace
        </h1>
        <p className="mt-2 text-center text-sm text-muted-foreground">
          Select a workspace to start a new session.
        </p>

        <div className="mt-8 space-y-3">
          <Button
            variant="outline"
            size="lg"
            className="w-full justify-center"
            onClick={onChooseFolder}
          >
            Choose folder...
          </Button>

          {selected?.type === "picked" && (
            <div
              data-workspace-selected="true"
              className="rounded-2xl border border-primary/35 bg-primary/8 px-4 py-3"
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
            <div className="space-y-2">
              <div className="text-xs uppercase tracking-[0.18em] text-muted-foreground/70">
                Recent workspaces
              </div>
              <div className="space-y-2">
                {recentWorkspaces.map((workspace) => {
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
                        {shortenPath(workspace)}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {actionError && (
          <div className="mt-4 rounded-2xl border border-destructive/25 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {actionError}
          </div>
        )}

        <Button
          size="lg"
          className="mt-6 w-full"
          disabled={!selected}
          onClick={onContinue}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}
