import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { CyberSelect } from "@/components/ui/cyber-select";

const ALL_ROLES = [
  { value: "user", label: "User (Admin)" },
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
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
  const setRole = useBridgeStore((s) => s.setRole);

  const role = agent === "claude" ? claudeRole : codexRole;
  const otherRole = agent === "claude" ? codexRole : claudeRole;

  // Filter out the role already taken by the other agent
  const options = useMemo(
    () => ALL_ROLES.filter((r) => r.value === role || r.value !== otherRole),
    [role, otherRole],
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
