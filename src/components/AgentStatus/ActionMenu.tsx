import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export interface ActionMenuItem {
  label: string;
  danger?: boolean;
  onClick: () => void;
}

export function ActionMenu({
  items,
  trigger,
}: {
  items: ActionMenuItem[];
  trigger?: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0 });

  const updatePos = useCallback(() => {
    if (!triggerRef.current) return;
    const rect = triggerRef.current.getBoundingClientRect();
    setPos({ top: rect.bottom + 4, left: rect.right });
  }, []);

  useEffect(() => {
    if (!open) return;
    updatePos();
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        triggerRef.current?.contains(target) ||
        menuRef.current?.contains(target)
      )
        return;
      setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, updatePos]);

  if (items.length === 0) return null;

  return (
    <>
      {trigger ? (
        <button
          ref={triggerRef}
          onClick={() => setOpen(!open)}
          aria-label="Actions"
        >
          {trigger}
        </button>
      ) : (
        <button
          ref={triggerRef}
          className="rounded-md px-1.5 py-0.5 text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground"
          onClick={() => setOpen(!open)}
          aria-label="Actions"
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="8" cy="3" r="1.5" />
            <circle cx="8" cy="8" r="1.5" />
            <circle cx="8" cy="13" r="1.5" />
          </svg>
        </button>
      )}
      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="fixed z-9999 min-w-24 max-w-48 max-h-64 overflow-y-auto rounded-lg border border-border/60 bg-popover py-1 shadow-lg"
            style={{
              top: pos.top,
              left: pos.left,
              transform: "translateX(-100%)",
            }}
          >
            {items.map((item) => (
              <button
                key={item.label}
                className={`w-full px-3 py-1.5 text-left text-[11px] transition-colors hover:bg-muted/40 ${item.danger ? "text-rose-400" : "text-popover-foreground"}`}
                onClick={() => {
                  item.onClick();
                  setOpen(false);
                }}
              >
                {item.label}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
}
