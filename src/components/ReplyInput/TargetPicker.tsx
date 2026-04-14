import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { useTaskStore } from "@/stores/task-store";
import { selectActiveTaskRoleOptions } from "@/stores/task-store/selectors";

export type Target = string;

const KNOWN_COLORS: Record<string, string> = {
  auto: "text-purple-400 border-purple-400/30",
  lead: "text-yellow-400 border-yellow-400/30",
  coder: "text-emerald-400 border-emerald-400/30",
};
const DEFAULT_COLOR = "text-sky-400 border-sky-400/30";

function colorFor(t: string) {
  return KNOWN_COLORS[t] ?? DEFAULT_COLOR;
}

interface TargetPickerProps {
  target: Target;
  setTarget: (t: Target) => void;
}

export function TargetPicker({ target, setTarget }: TargetPickerProps) {
  const options = useTaskStore(selectActiveTaskRoleOptions);
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
    <div className="relative shrink-0" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className={`flex items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] font-medium transition-colors ${colorFor(target)}`}
      >
        To {target}
        <ChevronDown className="size-3 opacity-60" />
      </button>
      {open && (
        <div className="absolute bottom-full left-0 z-50 mb-2 min-w-[110px] rounded-xl border border-border bg-popover py-1 shadow-xl">
          {options.map((t) => (
            <button
              key={t}
              onClick={() => {
                setTarget(t);
                setOpen(false);
              }}
              className={`block w-full px-3 py-1.5 text-left text-[11px] transition-colors hover:bg-accent ${t === target ? "font-bold" : ""} ${colorFor(t).split(" ")[0]}`}
            >
              {t}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
