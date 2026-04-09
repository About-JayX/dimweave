import { useEffect, useRef, useState } from "react";

export interface ActionMenuItem {
  label: string;
  danger?: boolean;
  onClick: () => void;
}

export function ActionMenu({ items }: { items: ActionMenuItem[] }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  if (items.length === 0) return null;

  return (
    <div ref={ref} className="relative">
      <button
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
      {open && (
        <div className="absolute right-0 top-full z-50 mt-1 min-w-30 rounded-lg border border-border/60 bg-popover py-1 shadow-lg">
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
        </div>
      )}
    </div>
  );
}
