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

  const assigneeMenu: ActionMenuItem[] = [
    ...(assigneeFilter
      ? [{ label: "全部经办人", onClick: () => onAssigneeChange("") }]
      : []),
    ...teamMembers
      .filter((a) => a !== assigneeFilter)
      .map((a) => ({
        label: a,
        onClick: () => onAssigneeChange(a),
      })),
  ];

  const statusMenu: ActionMenuItem[] = [
    ...(statusFilter
      ? [{ label: "全部状态", onClick: () => onStatusChange?.("") }]
      : []),
    ...statusOptions
      .filter((s) => s !== statusFilter)
      .map((s) => ({
        label: s,
        onClick: () => onStatusChange?.(s),
      })),
  ];

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
