import type { FeishuSyncMode } from "@/stores/feishu-project-store";
import {
  ActionMenu,
  type ActionMenuItem,
} from "@/components/AgentStatus/ActionMenu";

interface SyncModeNavProps {
  className?: string;
  value: FeishuSyncMode;
  onChange: (mode: FeishuSyncMode) => void;
  disabled?: boolean;
  teamMembers: string[];
  assigneeFilter: string;
  onAssigneeChange: (assignee: string) => void;
  statusOptions?: string[];
  statusFilter?: string;
  onStatusChange?: (status: string) => void;
}

const MODE_LABELS: Record<FeishuSyncMode, string> = {
  todo: "我的待办",
  issues: "缺陷管理",
};

function DropdownTrigger({ label }: { label: string }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-md border border-border/50 bg-card/50 px-1.5 py-0.5 text-[11px] text-foreground transition-colors hover:bg-muted/40">
      {label}
      <svg
        width="10"
        height="10"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      >
        <path d="M4 6l4 4 4-4" />
      </svg>
    </span>
  );
}

export function buildStatusMenu(
  options: string[],
  selected: string,
  onChange: (s: string) => void,
): ActionMenuItem[] {
  return [
    ...(selected ? [{ label: "全部状态", onClick: () => onChange("") }] : []),
    ...options.map((s) => ({
      label: s,
      active: s === selected,
      onClick: () => onChange(s),
    })),
  ];
}

export function buildAssigneeMenu(
  members: string[],
  selected: string,
  onChange: (a: string) => void,
): ActionMenuItem[] {
  return [
    ...(selected ? [{ label: "全部经办人", onClick: () => onChange("") }] : []),
    ...members.map((a) => ({
      label: a,
      active: a === selected,
      onClick: () => onChange(a),
    })),
  ];
}

export function SyncModeNav({
  className,
  value,
  onChange,
  disabled,
  teamMembers,
  assigneeFilter,
  onAssigneeChange,
  statusOptions = [],
  statusFilter = "",
  onStatusChange,
}: SyncModeNavProps) {
  const modeMenu: ActionMenuItem[] = (
    Object.entries(MODE_LABELS) as [FeishuSyncMode, string][]
  )
    .filter(([k]) => k !== value)
    .map(([k, label]) => ({
      label,
      onClick: () => {
        if (!disabled) onChange(k);
      },
    }));

  const assigneeMenu = buildAssigneeMenu(teamMembers, assigneeFilter, onAssigneeChange);

  const statusMenu = buildStatusMenu(statusOptions, statusFilter, onStatusChange ?? (() => {}));

  return (
    <div className={`flex items-center gap-1.5 ${className ?? ""}`}>
      <ActionMenu
        items={modeMenu}
        trigger={<DropdownTrigger label={MODE_LABELS[value]} />}
      />
      {statusOptions.length > 0 && (
        <ActionMenu
          items={statusMenu}
          trigger={<DropdownTrigger label={statusFilter || "全部状态"} />}
        />
      )}
      {teamMembers.length > 0 && (
        <ActionMenu
          items={assigneeMenu}
          trigger={<DropdownTrigger label={assigneeFilter || "全部经办人"} />}
        />
      )}
    </div>
  );
}
