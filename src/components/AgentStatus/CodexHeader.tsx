import { RoleSelect } from "./RoleSelect";
import { StatusDot } from "./StatusDot";

interface CodexHeaderProps {
  running: boolean;
  connectionLabel: string | null;
}

export function CodexHeader({ running, connectionLabel }: CodexHeaderProps) {
  return (
    <>
      <div className="flex items-center gap-2">
        <StatusDot
          status={running ? "connected" : "disconnected"}
          variant="codex"
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Codex
        </span>
        <RoleSelect agent="codex" disabled={running} />
        <span
          key={running ? "c" : "d"}
          className="text-[11px] uppercase text-secondary-foreground status-flash"
        >
          {running ? "connected" : "disconnected"}
        </span>
      </div>

      {connectionLabel && (
        <div className="mt-1 font-mono text-[11px] text-muted-foreground/80">
          {connectionLabel}
        </div>
      )}
    </>
  );
}
