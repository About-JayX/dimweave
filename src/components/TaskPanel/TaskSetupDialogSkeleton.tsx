import { useCallback, useEffect } from "react";

interface Props {
  mode: "create" | "edit";
  onClose: () => void;
}

/// Blocking skeleton shown while TaskSetupDialog's async deps (provider
/// history, Claude/Codex model lists) are still loading. Keeps the dialog
/// modal visually present so the user doesn't see an empty-dropdown flash.
export function TaskSetupDialogSkeleton({ mode, onClose }: Props) {
  const handleClose = useCallback(() => onClose(), [onClose]);
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", h);
    return () => document.removeEventListener("keydown", h);
  }, [handleClose]);

  return (
    <div
      data-dialog-skeleton="true"
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
    >
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={handleClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-busy="true"
        className="relative z-10 flex flex-col w-full max-w-2xl max-h-[90vh] rounded-xl border border-border/50 bg-card shadow-xl"
      >
        <div className="shrink-0 px-4 pt-4 pb-2">
          <h3 className="text-sm font-semibold text-foreground">
            {mode === "edit" ? "Edit Task" : "New Task"}
          </h3>
        </div>
        <div className="min-h-0 flex-1 flex overflow-hidden border-t border-border/30">
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
        <div className="shrink-0 flex items-center gap-2 border-t border-border/30 px-4 py-3">
          <div className="flex-1" />
          <div className="h-7 w-14 rounded bg-muted/30 animate-pulse" />
          <div className="h-7 w-24 rounded bg-muted/40 animate-pulse" />
          <div className="h-7 w-14 rounded bg-muted/40 animate-pulse" />
        </div>
      </div>
    </div>
  );
}
