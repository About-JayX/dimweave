import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { CyberSelect } from "@/components/ui/cyber-select";

// "user" is reserved for the human administrator and not assignable to agents.
const ALL_ROLES = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
];

export function RoleSelect({
  agent,
  disabled,
}: {
  agent: "claude" | "codex";
  disabled?: boolean;
}) {
  const claudeRole = useBridgeStore((s) => s.claudeRole);
  const codexRole = useBridgeStore((s) => s.codexRole);
  const agents = useBridgeStore((s) => s.agents);
  const setRole = useBridgeStore((s) => s.setRole);

  const role = agent === "claude" ? claudeRole : codexRole;
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
      onChange={(v) => setRole(agent, v)}
      disabled={disabled}
    />
  );
}
