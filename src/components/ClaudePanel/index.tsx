import { useState, useCallback, useEffect, useMemo } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { CyberSelect } from "@/components/ui/cyber-select";
import { invoke } from "@tauri-apps/api/core";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useTaskStore } from "@/stores/task-store";
import type { ProviderSessionInfo } from "@/types";
import { RoleSelect } from "@/components/AgentStatus/RoleSelect";
import { StatusDot } from "@/components/AgentStatus/StatusDot";
import {
  buildProviderHistoryOptions,
  findProviderHistoryEntry,
  formatProviderConnectionLabel,
  NEW_PROVIDER_SESSION_VALUE,
  resolveProviderHistoryWorkspace,
} from "@/components/AgentStatus/provider-session-view-model";
import { DevConfirmDialog } from "./DevConfirmDialog";
import { ClaudeConfigRows } from "./ClaudeConfigRows";
import { ClaudeHint } from "./ClaudeHint";
import {
  rememberClaudeDevConfirm,
  shouldPromptForClaudeDevConfirm,
} from "./dev-confirm";

interface ClaudePanelProps {
  connected: boolean;
  terminalRunning: boolean;
  providerSession?: ProviderSessionInfo;
}

export function ClaudePanel({
  connected,
  terminalRunning,
  providerSession,
}: ClaudePanelProps) {
  const [cwd, setCwd] = useState("");
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const [showDevConfirm, setShowDevConfirm] = useState(false);
  const [rememberChoice, setRememberChoice] = useState(true);
  const [selectedHistoryId, setSelectedHistoryId] = useState(
    NEW_PROVIDER_SESSION_VALUE,
  );
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const fetchProviderHistory = useTaskStore((s) => s.fetchProviderHistory);
  const providerHistory = useTaskStore((s) => s.providerHistory);
  const providerHistoryLoading = useTaskStore((s) => s.providerHistoryLoading);
  const providerHistoryError = useTaskStore((s) => s.providerHistoryError);
  const effectiveCwd = useMemo(
    () => resolveProviderHistoryWorkspace(cwd, providerSession),
    [cwd, providerSession],
  );

  const workspaceHistory = effectiveCwd
    ? (providerHistory[effectiveCwd] ?? [])
    : [];
  const historyOptions = useMemo(
    () => buildProviderHistoryOptions("claude", workspaceHistory),
    [workspaceHistory],
  );
  const selectedHistory = useMemo(
    () =>
      findProviderHistoryEntry("claude", workspaceHistory, selectedHistoryId),
    [selectedHistoryId, workspaceHistory],
  );
  const historyLoading = effectiveCwd
    ? providerHistoryLoading[effectiveCwd]
    : false;
  const historyError = effectiveCwd ? providerHistoryError[effectiveCwd] : null;
  const connectionLabel = useMemo(
    () => formatProviderConnectionLabel(providerSession),
    [providerSession],
  );

  useEffect(() => {
    if (!connected && !terminalRunning) {
      setDisconnecting(false);
    }
  }, [connected, terminalRunning]);

  useEffect(() => {
    if (!effectiveCwd) return;
    void fetchProviderHistory(effectiveCwd).catch(() => {});
  }, [effectiveCwd, fetchProviderHistory]);

  useEffect(() => {
    if (selectedHistoryId !== NEW_PROVIDER_SESSION_VALUE && !selectedHistory) {
      setSelectedHistoryId(NEW_PROVIDER_SESSION_VALUE);
    }
  }, [selectedHistory, selectedHistoryId]);

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) {
      setCwd(dir);
      setSelectedHistoryId(NEW_PROVIDER_SESSION_VALUE);
    }
  }, [pickDirectory]);

  const doLaunch = useCallback(async () => {
    setConnecting(true);
    try {
      setActionError(null);
      await invoke("daemon_launch_claude_sdk", {
        roleId: "lead",
        cwd: effectiveCwd,
        model: model || null,
        resumeSessionId: selectedHistory?.externalId ?? null,
      });
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setConnecting(false);
    }
  }, [effectiveCwd, model, selectedHistory]);

  const handleLaunch = useCallback(async () => {
    if (!effectiveCwd) return;
    // SDK mode: no dev-confirm needed (no channel prompt)
    doLaunch();
  }, [effectiveCwd, doLaunch]);

  const confirmDevLaunch = useCallback(async () => {
    if (rememberChoice) {
      rememberClaudeDevConfirm(effectiveCwd);
    }
    setShowDevConfirm(false);
    doLaunch();
  }, [effectiveCwd, rememberChoice, doLaunch]);

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

  return (
    <>
      <div
        className={cn(
          "rounded-lg border bg-card p-3 card-depth transition-all duration-300",
          connected
            ? "border-claude/40 glow-claude-subtle border-glow-claude"
            : "border-input hover:border-input/80",
        )}
      >
        <div className="flex items-center gap-2">
          <StatusDot
            status={connected ? "connected" : "disconnected"}
            variant="claude"
          />
          <span className="flex-1 text-[13px] font-medium text-card-foreground">
            Claude Code
          </span>
          <RoleSelect
            agent="claude"
            disabled={
              connected || terminalRunning || connecting || disconnecting
            }
          />
          <span
            key={disconnecting ? "x" : connected ? "c" : "d"}
            className="text-[11px] uppercase text-secondary-foreground status-flash"
          >
            {disconnecting
              ? "disconnecting"
              : connected
                ? "connected"
                : terminalRunning
                  ? "starting"
                  : "disconnected"}
          </span>
        </div>

        {connectionLabel && (
          <div className="mt-1 font-mono text-[11px] text-muted-foreground/80">
            {connectionLabel}
          </div>
        )}

        <ClaudeConfigRows
          model={model}
          effort={effort}
          cwd={effectiveCwd}
          disabled={connected || terminalRunning || connecting || disconnecting}
          onModelChange={setModel}
          onEffortChange={setEffort}
          onPickDir={handlePickDir}
        />

        <div className="mt-2 flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">History</span>
          <CyberSelect
            value={selectedHistoryId}
            options={historyOptions}
            onChange={setSelectedHistoryId}
            disabled={
              connected ||
              terminalRunning ||
              connecting ||
              disconnecting ||
              !effectiveCwd
            }
            placeholder="New session"
          />
        </div>

        {connected && (
          <Button
            size="sm"
            variant="secondary"
            className="mt-2 w-full active:scale-[0.98] transition-all duration-200"
            disabled={disconnecting}
            onClick={handleDisconnect}
          >
            {disconnecting ? (
              <span className="flex items-center gap-2">
                <span className="size-3 rounded-full border-2 border-foreground/20 border-t-foreground animate-spin" />
                Disconnecting…
              </span>
            ) : (
              "Disconnect Claude"
            )}
          </Button>
        )}

        {!connected && (
          <Button
            size="sm"
            className="mt-2 w-full bg-claude text-white hover:bg-claude/90 hover:shadow-[0_0_16px_#8b5cf640] active:scale-[0.98] transition-all duration-200 btn-ripple"
            disabled={
              !effectiveCwd || terminalRunning || connecting || disconnecting
            }
            onClick={handleLaunch}
          >
            {connecting ? (
              <span className="flex items-center gap-2">
                <span className="size-3 rounded-full border-2 border-white/30 border-t-white animate-spin" />
                Connecting…
              </span>
            ) : terminalRunning ? (
              <span className="flex items-center gap-2">
                <span className="size-3 rounded-full border-2 border-white/30 border-t-white animate-spin" />
                Claude Starting…
              </span>
            ) : (
              "Connect Claude"
            )}
          </Button>
        )}

        <ClaudeHint
          connected={connected}
          cwd={effectiveCwd}
          terminalRunning={terminalRunning}
          disconnecting={disconnecting}
          actionError={actionError ?? historyError}
        />
        {!connected && effectiveCwd && historyLoading && (
          <div className="mt-1.5 text-center text-[10px] text-muted-foreground">
            Loading Claude history...
          </div>
        )}
      </div>

      {showDevConfirm && (
        <DevConfirmDialog
          cwd={effectiveCwd}
          rememberChoice={rememberChoice}
          onRememberChoiceChange={setRememberChoice}
          onCancel={() => setShowDevConfirm(false)}
          onConfirm={confirmDevLaunch}
        />
      )}
    </>
  );
}
