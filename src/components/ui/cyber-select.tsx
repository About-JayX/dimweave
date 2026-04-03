import { useState, useRef, useEffect } from "react";
import { cn } from "@/lib/utils";

export interface CyberSelectOption {
  value: string;
  label: string;
  description?: string;
}

interface CyberSelectProps {
  value: string;
  options: CyberSelectOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function CyberSelect({
  value,
  options,
  onChange,
  disabled = false,
  placeholder,
}: CyberSelectProps) {
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

  const selected = options.find((o) => o.value === value);
  const displayLabel = selected?.label ?? placeholder ?? value;

  return (
    <div ref={ref} className="relative inline-flex">
      <button
        type="button"
        onClick={() => !disabled && setOpen(!open)}
        className={cn(
          "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium border outline-none transition-colors duration-200",
          disabled
            ? "opacity-50 cursor-not-allowed border-input bg-muted text-foreground/60"
            : open
              ? "border-primary/50 bg-muted/80 text-foreground ring-1 ring-primary/15"
              : "border-input bg-muted text-foreground hover:border-primary/40 hover:bg-muted/80 cursor-pointer",
        )}
      >
        <span className="truncate max-w-28">{displayLabel}</span>
        <svg
          width="8"
          height="8"
          viewBox="0 0 12 12"
          className={cn(
            "shrink-0 text-muted-foreground transition-transform duration-200",
            open && "rotate-180",
          )}
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
        <div className="absolute right-0 top-7 z-50 min-w-36 max-h-52 overflow-y-auto rounded-lg border border-border/60 bg-popover p-1 shadow-xl animate-in fade-in zoom-in-95 duration-150">
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => {
                onChange(opt.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full flex-col items-start rounded-md px-2.5 py-1.5 text-left text-[11px] transition-colors duration-150",
                "hover:bg-primary/10 hover:text-foreground",
                opt.value === value
                  ? "bg-primary/15 text-foreground"
                  : "text-foreground/80",
              )}
            >
              <span className="font-medium">{opt.label}</span>
              {opt.description && (
                <span className="text-[9px] text-muted-foreground mt-0.5">
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
