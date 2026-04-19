import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskAgents,
} from "@/stores/task-store/selectors";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { StatusDot } from "@/components/AgentStatus/StatusDot";
import type {
  TaskAgentInfo,
  AgentRuntimeStatus,
} from "@/stores/task-store/types";

function shortId(id: string | null | undefined, n = 8): string | null {
  if (!id) return null;
  return id.length <= n ? id : id.slice(0, n);
}

function AgentRow({
  agent,
  online,
  externalSessionId,
}: {
  agent: TaskAgentInfo;
  online: boolean;
  externalSessionId: string | null;
}) {
  const Icon = agent.provider === "claude" ? ClaudeIcon : CodexIcon;
  return (
    <div className="rounded-lg border border-border/30 bg-card/40 px-2.5 py-2 text-[11px]">
      <div className="flex items-center gap-1.5">
        <Icon className="size-3.5 shrink-0" />
        <span className="font-medium text-foreground capitalize">
          {agent.provider}
        </span>
        <span className="text-muted-foreground">{agent.role}</span>
        {agent.displayName && (
          <span className="truncate text-muted-foreground/70">
            · {agent.displayName}
          </span>
        )}
        <div className="ml-auto flex items-center gap-1">
          <StatusDot status={online ? "connected" : "disconnected"} />
          <span className="text-[10px] text-muted-foreground/70">
            {online ? "online" : "offline"}
          </span>
        </div>
      </div>
      <dl className="mt-1.5 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5 text-[10px] text-muted-foreground/80">
        <dt className="text-muted-foreground/60">Model</dt>
        <dd className="truncate text-foreground/80">
          {agent.model || "default"}
        </dd>
        <dt className="text-muted-foreground/60">Effort</dt>
        <dd className="truncate text-foreground/80">
          {agent.effort || "default"}
        </dd>
        {externalSessionId && (
          <>
            <dt className="text-muted-foreground/60">Session</dt>
            <dd
              className="truncate font-mono text-foreground/70"
              title={externalSessionId}
            >
              {shortId(externalSessionId)}
            </dd>
          </>
        )}
      </dl>
    </div>
  );
}

export function AgentsInfoPanel() {
  const task = useTaskStore(selectActiveTask);
  const agents = useTaskStore(selectActiveTaskAgents);
  const runtimeStatuses = useTaskStore((s) =>
    task ? s.agentRuntimeStatuses[task.taskId] : undefined,
  );
  const providerSummary = useTaskStore((s) =>
    task ? s.providerSummaries[task.taskId] : undefined,
  );

  if (!task) {
    return (
      <div className="rounded-lg border border-dashed border-border/40 px-3 py-4 text-center text-[11px] text-muted-foreground/60">
        No active task
      </div>
    );
  }
  if (agents.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border/40 px-3 py-4 text-center text-[11px] text-muted-foreground/60">
        No agents configured for this task
      </div>
    );
  }

  const onlineSet = new Set(
    (runtimeStatuses ?? [])
      .filter((s: AgentRuntimeStatus) => s.online)
      .map((s) => s.agentId),
  );
  const leadSessionId =
    providerSummary?.leadProviderSession?.externalSessionId ?? null;
  const coderSessionId =
    providerSummary?.coderProviderSession?.externalSessionId ?? null;

  return (
    <div className="space-y-1.5">
      {[...agents]
        .sort((a, b) => a.order - b.order)
        .map((agent) => {
          const externalSessionId =
            agent.role === "lead"
              ? leadSessionId
              : agent.role === "coder"
                ? coderSessionId
                : null;
          return (
            <AgentRow
              key={agent.agentId}
              agent={agent}
              online={onlineSet.has(agent.agentId)}
              externalSessionId={externalSessionId}
            />
          );
        })}
    </div>
  );
}
