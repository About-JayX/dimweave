import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { CyberSelect } from "@/components/ui/cyber-select";

// "user" is reserved for the human administrator and not assignable to agents.
export const AGENT_ROLE_OPTIONS = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
];
const ALL_ROLES = AGENT_ROLE_OPTIONS;

export function RoleSelect({
  agent,
  disabled,
  draftValue,
  onDraftChange,
}: {
  agent: "claude" | "codex";
  disabled?: boolean;
  draftValue?: string;
  onDraftChange?: (value: string) => void;
}) {
  const claudeRole = useBridgeStore((s) => s.claudeRole);
  const codexRole = useBridgeStore((s) => s.codexRole);
  const agents = useBridgeStore((s) => s.agents);
  const setRole = useBridgeStore((s) => s.setRole);

  const role = draftValue ?? (agent === "claude" ? claudeRole : codexRole);
  const otherAgent = agent === "claude" ? "codex" : "claude";
  const otherRole = agent === "claude" ? codexRole : claudeRole;
  const otherOnline = agents[otherAgent]?.status === "connected";

  // Only block the other agent's role while that agent is actually online.
  const options = useMemo(
    () =>
      ALL_ROLES.filter(
        (r) => !otherOnline || r.value === role || r.value !== otherRole,
      ),
    [otherOnline, otherRole, role],
  );

  return (
    <CyberSelect
      value={role}
      options={options}
      onChange={onDraftChange ?? ((v) => setRole(agent, v))}
      disabled={disabled}
      placeholder={!role ? "Select role" : undefined}
    />
  );
}
