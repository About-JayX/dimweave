import { useCallback, useEffect, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/// Shared chrome for modal dialogs: fixed header / scrollable body / fixed footer.
/// ProviderAuthDialog and TaskSetupDialog used to hard-code `max-h-[90vh]`,
/// `overflow-hidden`, and shrink-0 header/footer each on their own — drift
/// between the two caused footer-clipping and height-mismatch regressions
/// (see commits `8458800` and `454ee30`). One component now owns:
///   - outer overlay + click-to-close
///   - `max-h-[90vh]` cap (with optional min/max width override)
///   - fixed header, flex-1 min-h-0 body (scroll container), fixed footer
///   - ESC key to close
/// Consumers pass a `width` preset (sm/md/lg) to match their form density.

export type DialogWidth = "sm" | "md" | "lg";

const WIDTH_CLASS: Record<DialogWidth, string> = {
  sm: "max-w-md",
  md: "max-w-lg",
  lg: "max-w-2xl",
};

export interface DialogLayoutProps {
  open: boolean;
  onClose: () => void;
  /** Tailwind max-width preset (default "md"). */
  width?: DialogWidth;
  /** Optional fixed height cap; default: `max-h-[90vh]`. */
  heightClassName?: string;
  /** Rendered inside the fixed top strip. */
  header: ReactNode;
  /** Rendered in the scrollable body. Caller controls inner padding. */
  body: ReactNode;
  /** Optional fixed footer (buttons). Renders `border-t` when present. */
  footer?: ReactNode;
  /** Escape-hatch for extra classes on the dialog panel. */
  panelClassName?: string;
  /**
   * When true, body is a flex container (use for two-pane layouts). Default
   * false (single vertical scroll region). Two-pane callers need to handle
   * their own inner overflow.
   */
  bodyFlex?: boolean;
}

export function DialogLayout({
  open,
  onClose,
  width = "md",
  heightClassName = "max-h-[90vh]",
  header,
  body,
  footer,
  panelClassName,
  bodyFlex = false,
}: DialogLayoutProps) {
  const handleClose = useCallback(() => onClose(), [onClose]);
  useEffect(() => {
    if (!open) return;
    const h = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", h);
    return () => document.removeEventListener("keydown", h);
  }, [open, handleClose]);
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={handleClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          "relative z-10 flex flex-col w-full overflow-hidden rounded-xl border border-border/50 bg-card shadow-xl",
          WIDTH_CLASS[width],
          heightClassName,
          panelClassName,
        )}
      >
        <div className="shrink-0 border-b border-border/30 px-4 py-3">
          {header}
        </div>
        <div
          className={cn(
            "min-h-0 flex-1 overflow-hidden",
            bodyFlex ? "flex" : "overflow-y-auto",
          )}
        >
          {body}
        </div>
        {footer && (
          <div className="shrink-0 border-t border-border/30 px-4 py-3">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
