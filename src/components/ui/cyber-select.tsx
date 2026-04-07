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
  variant?: "default" | "history";
}

export function HistoryMenuOption({
  opt,
  isSelected,
  onClick,
}: {
  opt: CyberSelectOption;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-start rounded-md px-3 py-2 text-[12px] text-left transition-colors duration-150",
        "hover:bg-primary/10 hover:text-foreground",
        isSelected ? "bg-primary/15 text-foreground" : "text-foreground/80",
      )}
    >
      <div className="flex flex-col items-start w-full gap-0.5">
        <span className="font-medium break-words">{opt.label}</span>
        {opt.description && (
          <span className="text-[11px] text-muted-foreground/70 break-all">
            {opt.description}
          </span>
        )}
      </div>
    </button>
  );
}

export function CyberSelect({
  value,
  options,
  onChange,
  disabled = false,
  placeholder,
  variant = "default",
}: CyberSelectProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const isHistory = variant === "history";

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
          "inline-flex items-center gap-1 border outline-none transition-colors duration-200 font-medium",
          isHistory
            ? "min-w-[11rem] max-w-[15rem] justify-between rounded-full px-3 py-1 text-[11px]"
            : "rounded px-1.5 py-0.5 text-[10px]",
          disabled
            ? "opacity-50 cursor-not-allowed border-input bg-muted text-foreground/60"
            : open
              ? "border-primary/50 bg-muted/80 text-foreground ring-1 ring-primary/15"
              : "border-input bg-muted text-foreground hover:border-primary/40 hover:bg-muted/80 cursor-pointer",
        )}
      >
        {!isHistory && selected?.description ? (
          <div className="flex flex-col items-start min-w-0">
            <span className="truncate max-w-28 leading-tight">
              {displayLabel}
            </span>
            <span className="truncate max-w-28 text-[9px] text-muted-foreground/70 leading-tight">
              {selected.description}
            </span>
          </div>
        ) : (
          <span
            className={cn(
              "min-w-0 truncate text-left",
              isHistory ? "flex-1" : "max-w-28",
            )}
          >
            {displayLabel}
          </span>
        )}
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
        <div
          className={cn(
            "absolute right-0 z-50 overflow-y-auto border border-border/60 bg-popover shadow-xl animate-in fade-in zoom-in-95 duration-150",
            isHistory
              ? "top-8 w-[22rem] max-w-[min(22rem,calc(100vw-2rem))] max-h-64 rounded-2xl p-2"
              : "top-7 min-w-36 max-w-64 max-h-52 rounded-lg p-1",
          )}
        >
          {options.map((opt) =>
            isHistory ? (
              <HistoryMenuOption
                key={opt.value}
                opt={opt}
                isSelected={opt.value === value}
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
              />
            ) : (
              <button
                key={opt.value}
                type="button"
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
                className={cn(
                  "flex w-full items-start rounded-md px-2.5 py-1.5 text-[11px] text-left transition-colors duration-150",
                  "hover:bg-primary/10 hover:text-foreground",
                  opt.value === value
                    ? "bg-primary/15 text-foreground"
                    : "text-foreground/80",
                )}
              >
                <div className="flex flex-col items-start min-w-0 flex-1">
                  <span className="font-medium truncate w-full">{opt.label}</span>
                  {opt.description && (
                    <span className="text-[10px] text-muted-foreground/70 truncate w-full mt-0.5">
                      {opt.description}
                    </span>
                  )}
                </div>
              </button>
            ),
          )}
        </div>
      )}
    </div>
  );
}
