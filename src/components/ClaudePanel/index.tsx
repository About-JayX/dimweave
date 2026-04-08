import { useState, useCallback, useEffect, useMemo } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { CyberSelect } from "@/components/ui/cyber-select";
import { invoke } from "@tauri-apps/api/core";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import type { ProviderSessionInfo } from "@/types";
import { ClaudeIcon } from "@/components/AgentStatus/BrandIcons";
import { RoleSelect } from "@/components/AgentStatus/RoleSelect";
import { StatusDot } from "@/components/AgentStatus/StatusDot";
import {
  makeProviderHistoryErrorSelector,
  makeProviderHistoryLoadingSelector,
  makeProviderHistorySelector,
  selectActiveTask,
} from "@/stores/task-store/selectors";
import {
  buildProviderHistoryOptions,
  findProviderHistoryEntry,
  formatProviderConnectionLabel,
  NEW_PROVIDER_SESSION_VALUE,
  resolveProviderHistoryAction,
  resolveProviderHistoryWorkspace,
} from "@/components/AgentStatus/provider-session-view-model";
import { ClaudeConfigRows } from "./ClaudeConfigRows";
import { ClaudeHint } from "./ClaudeHint";
import { buildClaudeLaunchRequest } from "./launch-request";
import { ChevronDown, ChevronUp, SlidersHorizontal } from "lucide-react";

interface ClaudePanelProps {
  connected: boolean;
  providerSession?: ProviderSessionInfo;
}

export function ClaudePanel({ connected, providerSession }: ClaudePanelProps) {
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [selectedHistoryId, setSelectedHistoryId] = useState(
    NEW_PROVIDER_SESSION_VALUE,
  );
  const claudeRole = useBridgeStore((s) => s.claudeRole);
  const activeTask = useTaskStore(selectActiveTask);
  const fetchProviderHistory = useTaskStore((s) => s.fetchProviderHistory);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const effectiveCwd = useMemo(
    () => resolveProviderHistoryWorkspace(activeTask?.workspaceRoot),
    [activeTask?.workspaceRoot],
  );
  const selectWorkspaceHistory = useMemo(
    () => makeProviderHistorySelector(effectiveCwd),
    [effectiveCwd],
  );
  const selectWorkspaceHistoryLoading = useMemo(
    () => makeProviderHistoryLoadingSelector(effectiveCwd),
    [effectiveCwd],
  );
  const selectWorkspaceHistoryError = useMemo(
    () => makeProviderHistoryErrorSelector(effectiveCwd),
    [effectiveCwd],
  );
  const workspaceHistory = useTaskStore(selectWorkspaceHistory);
  const historyLoading = useTaskStore(selectWorkspaceHistoryLoading);
  const historyError = useTaskStore(selectWorkspaceHistoryError);

  const historyOptions = useMemo(
    () => buildProviderHistoryOptions("claude", workspaceHistory),
    [workspaceHistory],
  );
  const selectedHistory = useMemo(
    () =>
      findProviderHistoryEntry("claude", workspaceHistory, selectedHistoryId),
    [selectedHistoryId, workspaceHistory],
  );
  const connectionLabel = useMemo(
    () => formatProviderConnectionLabel(providerSession),
    [providerSession],
  );

  useEffect(() => {
    if (!connected) {
      setDisconnecting(false);
    }
  }, [connected]);

  useEffect(() => {
    if (connected) {
      setShowAdvanced(false);
    }
  }, [connected]);

  useEffect(() => {
    if (!effectiveCwd) return;
    void fetchProviderHistory(effectiveCwd).catch(() => {});
  }, [effectiveCwd, fetchProviderHistory]);

  useEffect(() => {
    if (selectedHistoryId !== NEW_PROVIDER_SESSION_VALUE && !selectedHistory) {
      setSelectedHistoryId(NEW_PROVIDER_SESSION_VALUE);
    }
  }, [selectedHistory, selectedHistoryId]);

  const doLaunch = useCallback(async () => {
    setConnecting(true);
    try {
      setActionError(null);
      const action = resolveProviderHistoryAction(selectedHistory);
      if (action.kind === "resumeNormalized") {
        await resumeSession(action.sessionId);
      } else {
        await invoke(
          "daemon_launch_claude_sdk",
          buildClaudeLaunchRequest({
            claudeRole,
            cwd: effectiveCwd,
            model,
            effort,
            resumeSessionId:
              action.kind === "resumeExternal" ? action.externalId : undefined,
          }),
        );
      }
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setConnecting(false);
    }
  }, [claudeRole, effectiveCwd, model, effort, selectedHistory, resumeSession]);

  const handleLaunch = useCallback(async () => {
    if (!effectiveCwd) return;
    await doLaunch();
  }, [effectiveCwd, doLaunch]);

  const handleDisconnect = useCallback(async () => {
    setDisconnecting(true);
    setActionError(null);
    try {
      await invoke("stop_claude");
    } catch (e) {
      // Also try SDK-specific stop as fallback
      try {
        await invoke("daemon_stop_claude_sdk");
      } catch {
        /* ignore */
      }
      setDisconnecting(false);
      setActionError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const summaryChips = useMemo(
    () => [
      effectiveCwd
        ? effectiveCwd.split("/").pop() || effectiveCwd
        : "Workspace required",
      model || "Default model",
      effort || "Default effort",
      selectedHistory
        ? `Resume ${selectedHistory.externalId.slice(0, 12)}`
        : "New session",
    ],
    [effectiveCwd, effort, model, selectedHistory],
  );

  return (
    <div
      className={cn(
        "rounded-2xl border bg-card px-4 py-3 card-depth transition-colors",
        connected || connecting
          ? "border-claude/35 glow-claude-subtle border-glow-claude"
          : "border-input hover:border-input/80",
      )}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <StatusDot
            status={connected ? "connected" : "disconnected"}
            variant="claude"
          />
          <ClaudeIcon className="size-7 text-claude" />
        </div>
        <RoleSelect
          agent="claude"
          disabled={connected || connecting || disconnecting}
        />
      </div>

      {connectionLabel && (
        <span
          className="mt-1.5 inline-block cursor-pointer truncate rounded-full border border-claude/15 bg-claude/8 px-2.5 py-0.5 text-[10px] text-claude/70 transition-colors hover:bg-claude/15 hover:text-claude"
          title={connectionLabel.full}
        >
          {connectionLabel.short}
        </span>
      )}

      <div className="mt-3 flex flex-wrap gap-1.5">
        {summaryChips.map((chip) => (
          <span
            key={chip}
            className="rounded-full border border-border/45 bg-background/35 px-2 py-0.5 text-[10px] text-muted-foreground"
          >
            {chip}
          </span>
        ))}
      </div>

      <div className="mt-3 flex gap-2">
        {connected ? (
          <Button
            size="sm"
            variant="destructive"
            className="flex-1 rounded-full"
            disabled={disconnecting}
            onClick={handleDisconnect}
          >
            {disconnecting ? (
              <span className="flex items-center gap-2">
                <span className="size-3 rounded-full radius-keep border-2 border-foreground/20 border-t-foreground animate-spin" />
                Disconnecting…
              </span>
            ) : (
              "Disconnect"
            )}
          </Button>
        ) : (
          <Button
            size="sm"
            className="flex-1 rounded-full bg-claude/15 text-claude border-claude/25 hover:bg-claude/25"
            disabled={!effectiveCwd || connecting || disconnecting}
            onClick={handleLaunch}
          >
            {connecting ? (
              <span className="flex items-center gap-2">
                <span className="size-3 rounded-full radius-keep border-2 border-claude/30 border-t-claude animate-spin" />
                Connecting…
              </span>
            ) : (
              "Connect"
            )}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="shrink-0 text-muted-foreground"
          onClick={() => setShowAdvanced((open) => !open)}
        >
          <SlidersHorizontal className="size-3.5" />
          {showAdvanced ? (
            <ChevronUp className="size-3.5" />
          ) : (
            <ChevronDown className="size-3.5" />
          )}
        </Button>
      </div>

      <ClaudeHint
        connected={connected}
        cwd={effectiveCwd}
        disconnecting={disconnecting}
        actionError={actionError ?? historyError}
      />

      {showAdvanced && (
        <div className="mt-3 rounded-xl border border-border/35 bg-background/30 px-3 py-3">
          <ClaudeConfigRows
            model={model}
            effort={effort}
            disabled={connected || connecting || disconnecting}
            onModelChange={setModel}
            onEffortChange={setEffort}
          />

          <div className="mt-2 flex items-center gap-2">
            <span className="shrink-0 text-[10px] text-muted-foreground">
              History
            </span>
            <CyberSelect
              variant="history"
              value={selectedHistoryId}
              options={historyOptions}
              onChange={setSelectedHistoryId}
              disabled={
                connected || connecting || disconnecting || !effectiveCwd
              }
              placeholder="New session"
            />
          </div>

          {!connected && effectiveCwd && historyLoading && (
            <div className="mt-1.5 text-center text-[10px] text-muted-foreground">
              Loading Claude history...
            </div>
          )}
        </div>
      )}
    </div>
  );
}
