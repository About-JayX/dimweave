import { CodexIcon } from "./BrandIcons";
import { RoleSelect } from "./RoleSelect";
import { StatusDot } from "./StatusDot";
import type { ConnectionLabel } from "./provider-session-view-model";

interface CodexHeaderProps {
  running: boolean;
  connectionLabel: ConnectionLabel | null;
  draftRole?: string;
  onDraftRoleChange?: (role: string) => void;
}

export function CodexHeader({
  running,
  connectionLabel,
  draftRole,
  onDraftRoleChange,
}: CodexHeaderProps) {
  return (
    <>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <StatusDot
            status={running ? "connected" : "disconnected"}
            variant="codex"
          />
          <CodexIcon className="size-7 text-codex" />
        </div>
        <RoleSelect agent="codex" disabled={running} draftValue={draftRole} onDraftChange={onDraftRoleChange} />
      </div>

      {connectionLabel && (
        <span
          className="mt-1.5 inline-block cursor-pointer truncate rounded-full border border-codex/15 bg-codex/8 px-2.5 py-0.5 text-[10px] text-codex/70 transition-colors hover:bg-codex/15 hover:text-codex"
          title={connectionLabel.full}
        >
          {connectionLabel.short}
        </span>
      )}
    </>
  );
}
