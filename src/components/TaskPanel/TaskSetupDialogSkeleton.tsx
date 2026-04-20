import { DialogLayout } from "@/components/ui/dialog-layout";

interface Props {
  mode: "create" | "edit";
  onClose: () => void;
}

/// Blocking skeleton shown while TaskSetupDialog's async deps (provider
/// history, Claude/Codex model lists) are still loading. Keeps the dialog
/// modal visually present so the user doesn't see an empty-dropdown flash.
/// Uses DialogLayout to share chrome with the fully-loaded dialog; only
/// the body content is stubbed out.
export function TaskSetupDialogSkeleton({ mode, onClose }: Props) {
  return (
    <DialogLayout
      open
      onClose={onClose}
      width="lg"
      bodyFlex
      panelClassName="aria-busy"
      header={
        <h3 className="text-sm font-semibold text-foreground">
          {mode === "edit" ? "Edit Task" : "New Task"}
        </h3>
      }
      body={
        <div className="flex h-full w-full">
          <div className="flex w-52 shrink-0 flex-col border-r border-border/30">
            <div className="flex items-center justify-between px-3 py-2">
              <span className="h-3 w-12 rounded bg-muted/50 animate-pulse" />
              <span className="h-3 w-8 rounded bg-muted/40 animate-pulse" />
            </div>
            <div className="space-y-1.5 px-2 pb-2">
              {[0, 1, 2].map((i) => (
                <div
                  key={i}
                  className="h-9 rounded-md bg-muted/40 animate-pulse"
                />
              ))}
            </div>
          </div>
          <div className="flex flex-1 flex-col gap-3 overflow-y-auto p-4">
            {[0, 1, 2, 3].map((i) => (
              <div key={i} className="space-y-1.5">
                <div className="h-3 w-20 rounded bg-muted/40 animate-pulse" />
                <div className="h-8 rounded-md bg-muted/30 animate-pulse" />
              </div>
            ))}
          </div>
        </div>
      }
      footer={
        <div className="flex items-center gap-2">
          <div className="flex-1" />
          <div className="h-7 w-14 rounded bg-muted/30 animate-pulse" />
          <div className="h-7 w-24 rounded bg-muted/40 animate-pulse" />
          <div className="h-7 w-14 rounded bg-muted/40 animate-pulse" />
        </div>
      }
    />
  );
}
