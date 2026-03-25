import { useEffect, useState, useCallback, useRef } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { buildClaudeAgentsJson, buildMcpConfigJson } from "@/lib/agent-roles";
import { ConfigSelect } from "./ClaudeModelSelect";
import { ClaudeQuota } from "./ClaudeQuota";
import { useClaudeConfig } from "./useClaudeConfig";
import { shortenPath } from "./helpers";
import { RoleSelect } from "@/components/AgentStatus/RoleSelect";
import { StatusDot } from "@/components/AgentStatus/StatusDot";

interface ClaudePanelProps {
  connected: boolean;
}

export function ClaudePanel({ connected }: ClaudePanelProps) {
  const [cwd, setCwd] = useState("");
  const [model, setModel] = useState("sonnet");
  const [effort, setEffort] = useState("max");
  const [isRunning, setIsRunning] = useState(false);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const claudeRole = useBridgeStore((s) => s.claudeRole);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const { config: claudeConfig, loading: configLoading } = useClaudeConfig();

  const locked = connected || isRunning;
  const prevConnectedRef = useRef(connected);
  const [justConnected, setJustConnected] = useState(false);

  useEffect(() => {
    if (connected && !prevConnectedRef.current) {
      setJustConnected(true);
      const t = setTimeout(() => setJustConnected(false), 600);
      return () => clearTimeout(t);
    }
    prevConnectedRef.current = connected;
  }, [connected]);

  useEffect(() => {
    const unlisten = listen<number>("pty-exit", () => setIsRunning(false));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const handleLaunch = useCallback(async () => {
    if (!cwd) return;
    try {
      setLaunchError(null);
      const agentsJson = buildClaudeAgentsJson(claudeRole);
      const mcpConfigJson = buildMcpConfigJson(claudeConfig.bridgePath);
      await invoke("launch_pty", {
        cwd,
        cols: 120,
        rows: 30,
        roleId: claudeRole,
        agentsJson,
        mcpConfigJson,
        model,
        effort,
      });
      setIsRunning(true);
      window.dispatchEvent(new CustomEvent("switch-to-terminal"));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setLaunchError(msg);
    }
  }, [cwd, claudeRole, model, effort]);

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3 card-depth transition-all duration-300",
        connected
          ? "border-claude/40 glow-claude-subtle border-glow-claude"
          : isRunning
            ? "border-yellow-500/30"
            : "border-input hover:border-input/80",
        justConnected && "card-connect-anim",
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <StatusDot
          status={
            connected ? "connected" : isRunning ? "connecting" : "disconnected"
          }
          variant="claude"
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Claude Code
        </span>
        <RoleSelect agent="claude" disabled={locked} />
        {locked && (
          <button
            type="button"
            onClick={() =>
              window.dispatchEvent(new CustomEvent("switch-to-terminal"))
            }
            className="p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
            title="View terminal"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.3"
            >
              <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
              <path d="M4.5 7l2 1.5-2 1.5M8.5 11h3" />
            </svg>
          </button>
        )}
        <span
          key={connected ? "c" : isRunning ? "s" : "d"}
          className="text-[11px] uppercase text-secondary-foreground status-flash"
        >
          {connected ? "connected" : isRunning ? "starting" : "disconnected"}
        </span>
      </div>

      {/* Quota (when connected or running) */}
      {locked && <ClaudeQuota />}

      {/* Config rows — always visible, locked after connection */}
      <div className="mt-2 space-y-1.5">
        {/* CLI version */}
        {!configLoading && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">CLI</span>
            {claudeConfig.installed ? (
              <span
                className="text-[10px] text-secondary-foreground font-mono truncate max-w-48"
                title={claudeConfig.binaryPath}
              >
                {claudeConfig.version}
              </span>
            ) : (
              <span className="text-[10px] text-destructive">
                Not installed
              </span>
            )}
          </div>
        )}

        {/* Model */}
        {claudeConfig.installed && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Model</span>
            <ConfigSelect
              options={claudeConfig.models}
              value={model}
              onChange={setModel}
              disabled={locked}
            />
          </div>
        )}

        {/* Effort level */}
        {claudeConfig.installed && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Effort</span>
            <ConfigSelect
              options={claudeConfig.effortLevels}
              value={effort}
              onChange={setEffort}
              disabled={locked}
            />
          </div>
        )}

        {/* Project */}
        {claudeConfig.installed && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Project</span>
            <button
              type="button"
              onClick={handlePickDir}
              disabled={locked}
              className={cn(
                "inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground transition-colors truncate max-w-44",
                locked
                  ? "opacity-50 cursor-not-allowed"
                  : "hover:bg-accent hover:text-primary cursor-pointer",
              )}
              title={cwd}
            >
              <svg
                width="10"
                height="10"
                viewBox="0 0 16 16"
                className="shrink-0 text-muted-foreground"
              >
                <path
                  d="M2 4v8h12V6H8L6 4z"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.2"
                />
              </svg>
              {cwd ? shortenPath(cwd) : "Select project..."}
            </button>
          </div>
        )}
      </div>

      {/* Stop button */}
      {locked && (
        <Button
          size="xs"
          variant="destructive"
          className="w-full mt-2 active:scale-[0.98] transition-all duration-200"
          onClick={() => {
            invoke("stop_pty")
              .then(() => setIsRunning(false))
              .catch(console.warn);
          }}
        >
          Stop Claude
        </Button>
      )}

      {/* Launch button */}
      {!locked && claudeConfig.installed && (
        <Button
          size="sm"
          className="w-full mt-2 bg-claude text-white hover:bg-claude/90 hover:shadow-[0_0_16px_#8b5cf640] active:scale-[0.98] transition-all duration-200 btn-ripple"
          disabled={!cwd}
          onClick={handleLaunch}
        >
          Connect Claude
        </Button>
      )}

      {/* Status messages */}
      {!claudeConfig.installed && !configLoading && (
        <div className="mt-1.5 text-[10px] text-destructive text-center">
          Claude CLI not found. Install it first.
        </div>
      )}
      {claudeConfig.installed && !cwd && !locked && (
        <div className="mt-1.5 text-[10px] text-muted-foreground text-center">
          Select a project directory first
        </div>
      )}
      {launchError && (
        <div className="mt-1.5 text-[10px] text-destructive text-center">
          {launchError}
        </div>
      )}
    </div>
  );
}
