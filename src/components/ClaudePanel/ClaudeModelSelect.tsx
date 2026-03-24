import { cn } from "@/lib/utils";
import type { SelectOption } from "./useClaudeConfig";

interface Props {
  options: SelectOption[];
  value: string;
  onChange: (v: string) => void;
  disabled: boolean;
}

export function ConfigSelect({ options, value, onChange, disabled }: Props) {
  if (options.length === 0) return null;
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      className={cn(
        "rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-foreground border border-input outline-none",
        disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
      )}
    >
      {options.map((o) => (
        <option key={o.id} value={o.id}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
