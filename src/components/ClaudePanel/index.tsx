import { useState, useCallback, useEffect } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { RoleSelect } from "@/components/AgentStatus/RoleSelect";
import { StatusDot } from "@/components/AgentStatus/StatusDot";
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
}

export function ClaudePanel({ connected, terminalRunning }: ClaudePanelProps) {
  const [cwd, setCwd] = useState("");
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const [showDevConfirm, setShowDevConfirm] = useState(false);
  const [rememberChoice, setRememberChoice] = useState(true);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);

  useEffect(() => {
    if (!connected && !terminalRunning) {
      setDisconnecting(false);
    }
  }, [connected, terminalRunning]);

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const doLaunch = useCallback(async () => {
    setConnecting(true);
    try {
      setActionError(null);
      await invoke("register_mcp", { cwd });
      // Estimate terminal dimensions from current window layout.
      // Sidebar ~280px, terminal padding ~16px, char width ~7.8px for 13px Geist Mono,
      // line height ~15px. UI chrome (tabs + header + input) ~140px.
      const cols = Math.max(80, Math.floor((window.innerWidth - 296) / 7.8));
      const rows = Math.max(24, Math.floor((window.innerHeight - 140) / 15));
      await invoke("launch_claude_terminal", {
        cwd,
        model: model || null,
        effort: effort || null,
        cols,
        rows,
      });
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    } finally {
      setConnecting(false);
    }
  }, [cwd, model, effort]);

  const handleLaunch = useCallback(async () => {
    if (!cwd) return;
    if (shouldPromptForClaudeDevConfirm(cwd)) {
      setShowDevConfirm(true);
      return;
    }
    doLaunch();
  }, [cwd, doLaunch]);

  const confirmDevLaunch = useCallback(async () => {
    if (rememberChoice) {
      rememberClaudeDevConfirm(cwd);
    }
    setShowDevConfirm(false);
    doLaunch();
  }, [cwd, rememberChoice, doLaunch]);

  const handleDisconnect = useCallback(async () => {
    setDisconnecting(true);
    setActionError(null);
    try {
      await invoke("stop_claude");
    } catch (e) {
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

        <ClaudeConfigRows
          model={model}
          effort={effort}
          cwd={cwd}
          disabled={connected || terminalRunning || connecting || disconnecting}
          onModelChange={setModel}
          onEffortChange={setEffort}
          onPickDir={handlePickDir}
        />

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
            disabled={!cwd || terminalRunning || connecting || disconnecting}
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
          cwd={cwd}
          terminalRunning={terminalRunning}
          disconnecting={disconnecting}
          actionError={actionError}
        />
      </div>

      {showDevConfirm && (
        <DevConfirmDialog
          cwd={cwd}
          rememberChoice={rememberChoice}
          onRememberChoiceChange={setRememberChoice}
          onCancel={() => setShowDevConfirm(false)}
          onConfirm={confirmDevLaunch}
        />
      )}
    </>
  );
}
