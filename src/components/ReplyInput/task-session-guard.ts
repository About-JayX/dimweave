import type { AgentInfo } from "@/types";
import type {
  AgentRuntimeStatus,
  SessionInfo,
  TaskInfo,
} from "@/stores/task-store/types";
import type { Target } from "./TargetPicker";

const RECONNECT_WARNING = "Reconnect to this task";
const LAUNCH_WARNING = "Launch agent for this task";

function roleRequiresTaskSession(role: string): role is "lead" | "coder" {
  return role === "lead" || role === "coder";
}

function resolveTargetRoles(
  target: Target,
  claudeRole: string,
  codexRole: string,
): string[] {
  if (target !== "auto") {
    return [target];
  }

  const roles = [claudeRole, codexRole].filter(
    (role) => role !== "user" && role.trim().length > 0,
  );
  return Array.from(new Set(roles));
}

function taskSessionForRole(
  activeTask: TaskInfo | null,
  sessions: SessionInfo[],
  role: "lead" | "coder",
): SessionInfo | null {
  if (!activeTask) {
    return null;
  }

  const expectedSessionId =
    role === "lead"
      ? activeTask.leadSessionId
      : activeTask.currentCoderSessionId;
  if (!expectedSessionId) {
    return null;
  }

  return (
    sessions.find((session) => session.sessionId === expectedSessionId) ?? null
  );
}

function taskTargetMatchState({
  agentId,
  agent,
  role,
  activeTask,
  sessions,
}: {
  agentId: "claude" | "codex";
  agent: AgentInfo | undefined;
  role: string;
  activeTask: TaskInfo | null;
  sessions: SessionInfo[];
}) {
  if (agent?.status !== "connected") {
    return "mismatch" as const;
  }

  if (!roleRequiresTaskSession(role)) {
    return "match" as const;
  }

  const expectedSession = taskSessionForRole(activeTask, sessions, role);
  if (!expectedSession || !expectedSession.externalSessionId) {
    return "needs-launch" as const;
  }

  const providerSession = agent.providerSession;
  if (!providerSession) {
    return "needs-launch" as const;
  }

  const expectedProvider = agentId === "claude" ? "claude" : "codex";
  const matches =
    expectedSession.provider === expectedProvider &&
    providerSession.provider === expectedProvider &&
    providerSession.externalSessionId === expectedSession.externalSessionId;
  return matches ? ("match" as const) : ("mismatch" as const);
}

export function getTaskSessionWarning({
  target,
  activeTask,
  sessions,
  agents,
  claudeRole,
  codexRole,
  taskRuntimeStatuses,
}: {
  target: Target;
  activeTask: TaskInfo | null;
  sessions: SessionInfo[];
  agents: Record<string, AgentInfo>;
  claudeRole: string;
  codexRole: string;
  /** Per-task agent runtime status, indexed by agentId.
   *  Source of truth for "is this task's agent online", immune to the
   *  singleton agents[] map that only reflects the last-launched provider. */
  taskRuntimeStatuses?: AgentRuntimeStatus[];
}) {
  if (!activeTask) {
    return null;
  }

  // Per-task first: if any agent belonging to the active task is online,
  // the ReplyInput can send. The singleton `agents.claude`/`agents.codex`
  // providerSession may still be pinned to another task's last launch,
  // but that's cosmetic — multi-task routing dispatches via
  // (task_id, agent_id) per the daemon's task_graph resolution.
  const perTaskOnline =
    (taskRuntimeStatuses ?? []).some((s) => s.online) === true;
  if (perTaskOnline) {
    return null;
  }

  const anyConnected =
    agents.claude?.status === "connected" ||
    agents.codex?.status === "connected";
  if (!anyConnected) {
    return null;
  }

  let needsLaunch = false;
  const roles = resolveTargetRoles(target, claudeRole, codexRole);
  const hasCompatibleTarget = roles.some((role) => {
    const claudeState =
      claudeRole === role
        ? taskTargetMatchState({
            agentId: "claude",
            agent: agents.claude,
            role,
            activeTask,
            sessions,
          })
        : null;
    const codexState =
      codexRole === role
        ? taskTargetMatchState({
            agentId: "codex",
            agent: agents.codex,
            role,
            activeTask,
            sessions,
          })
        : null;

    if (claudeState === "needs-launch" || codexState === "needs-launch") {
      needsLaunch = true;
    }

    return claudeState === "match" || codexState === "match";
  });

  if (hasCompatibleTarget) {
    return null;
  }

  return needsLaunch ? LAUNCH_WARNING : RECONNECT_WARNING;
}
