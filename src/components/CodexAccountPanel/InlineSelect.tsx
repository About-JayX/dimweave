import { useState, useRef, useEffect } from "react";
import { cn } from "@/lib/utils";
import type { DropdownOption } from "./types";

export function InlineSelect({
  value,
  options,
  onSelect,
  disabled = false,
}: {
  value: string;
  options: DropdownOption[];
  onSelect: (value: string) => void;
  disabled?: boolean;
}) {
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

  return (
    <div ref={ref} className="relative inline-flex">
      <button
        type="button"
        onClick={() => !disabled && setOpen(!open)}
        className={cn(
          "inline-flex items-center gap-0.5 rounded px-1 py-0.5 font-mono text-[11px] font-medium text-foreground transition-colors",
          disabled
            ? "opacity-50 cursor-not-allowed"
            : "hover:bg-accent cursor-pointer",
        )}
      >
        {value}
        <svg
          width="8"
          height="8"
          viewBox="0 0 12 12"
          className="text-muted-foreground"
        >
          <path
            d="M3 5l3 3 3-3"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          />
        </svg>
      </button>
      {open && (
        <div className="absolute right-0 top-6 z-50 min-w-40 max-h-48 overflow-y-auto rounded-lg border border-border bg-popover p-1 shadow-xl">
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => {
                onSelect(opt.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full flex-col items-start rounded-md px-2.5 py-1.5 text-left text-[11px] transition-colors",
                "hover:bg-accent hover:text-accent-foreground",
                opt.value === value && "bg-accent/60 text-accent-foreground",
              )}
            >
              <span className="font-medium">{opt.label}</span>
              {opt.description && (
                <span className="text-[10px] text-muted-foreground">
                  {opt.description}
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
