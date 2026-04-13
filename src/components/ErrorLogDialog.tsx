import type { UiError } from "@/stores/bridge-store/types";

interface ErrorLogDialogProps {
  open: boolean;
  errors: UiError[];
  onClose: () => void;
  onClear: () => void;
}

export function ErrorLogDialog({
  open,
  errors,
  onClose,
  onClear,
}: ErrorLogDialogProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        className="relative z-10 w-full max-w-md max-h-[70vh] overflow-y-auto rounded-xl border border-border/50 bg-card p-4 shadow-xl space-y-3"
      >
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-foreground">Error Log</h3>
          <div className="flex items-center gap-2">
            {errors.length > 0 && (
              <button
                type="button"
                onClick={onClear}
                className="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
              >
                Clear all
              </button>
            )}
            <button
              type="button"
              onClick={onClose}
              className="text-muted-foreground hover:text-foreground transition-colors text-sm leading-none"
            >
              &times;
            </button>
          </div>
        </div>

        {errors.length === 0 ? (
          <p className="text-xs text-muted-foreground py-4 text-center">
            No errors recorded
          </p>
        ) : (
          <div className="space-y-2">
            {errors.map((err) => (
              <div
                key={err.id}
                className="rounded-lg border border-destructive/20 bg-destructive/5 px-3 py-2"
              >
                <p className="text-xs text-destructive">{err.message}</p>
                {err.componentStack && (
                  <pre className="mt-1 text-[10px] text-muted-foreground/70 overflow-x-auto whitespace-pre-wrap">
                    {err.componentStack}
                  </pre>
                )}
                <span className="mt-1 block text-[9px] text-muted-foreground/50">
                  {new Date(err.timestamp).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
