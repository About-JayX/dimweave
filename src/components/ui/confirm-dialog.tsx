import { useCallback, useEffect } from "react";

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open, title, description,
  confirmLabel = "Delete",
  cancelLabel = "Cancel",
  onConfirm, onCancel,
}: ConfirmDialogProps) {
  const handleCancel = useCallback(() => onCancel(), [onCancel]);

  useEffect(() => {
    if (!open) return;
    const h = (e: KeyboardEvent) => { if (e.key === "Escape") handleCancel(); };
    document.addEventListener("keydown", h);
    return () => document.removeEventListener("keydown", h);
  }, [open, handleCancel]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={handleCancel} />
      <div role="alertdialog" aria-modal="true" data-confirm-dialog="true"
        className="relative z-10 w-full max-w-sm rounded-xl border border-border/50 bg-card p-5 shadow-xl">
        <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        <p className="mt-2 text-xs text-muted-foreground">{description}</p>
        <div className="mt-4 flex items-center justify-end gap-2">
          <button type="button" data-confirm-cancel="true" onClick={handleCancel}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground active:opacity-70">
            {cancelLabel}</button>
          <button type="button" data-confirm-action="true" onClick={onConfirm}
            className="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-rose-700 active:opacity-70">
            {confirmLabel}</button>
        </div>
      </div>
    </div>
  );
}
