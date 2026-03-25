import { useState, useCallback, useMemo } from "react";
import { cn, shortenPath } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { CyberSelect } from "@/components/ui/cyber-select";
import { invoke } from "@tauri-apps/api/core";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { RoleSelect } from "@/components/AgentStatus/RoleSelect";
import { StatusDot } from "@/components/AgentStatus/StatusDot";

const MODEL_OPTIONS = [
  { value: "", label: "Default" },
  { value: "sonnet", label: "Sonnet (latest)" },
  { value: "opus", label: "Opus (latest)" },
  { value: "claude-sonnet-4-6", label: "Sonnet 4.6" },
  { value: "claude-opus-4-6", label: "Opus 4.6" },
  { value: "claude-haiku-4-5", label: "Haiku 4.5" },
];

const EFFORT_OPTIONS = [
  { value: "", label: "Default" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "max", label: "Max (Opus only)" },
];

interface ClaudePanelProps {
  connected: boolean;
}

export function ClaudePanel({ connected }: ClaudePanelProps) {
  const [cwd, setCwd] = useState("");
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const handleLaunch = useCallback(async () => {
    if (!cwd) return;
    setConnecting(true);
    try {
      setLaunchError(null);
      await invoke("register_mcp", { cwd });
      await invoke("launch_claude_terminal", {
        cwd,
        model: model || null,
        effort: effort || null,
      });
    } catch (e) {
      setLaunchError(e instanceof Error ? e.message : String(e));
    } finally {
      setConnecting(false);
    }
  }, [cwd, model, effort]);

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3 card-depth transition-all duration-300",
        connected
          ? "border-claude/40 glow-claude-subtle border-glow-claude"
          : "border-input hover:border-input/80",
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <StatusDot
          status={connected ? "connected" : "disconnected"}
          variant="claude"
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Claude Code
        </span>
        <RoleSelect agent="claude" disabled={connected} />
        <span
          key={connected ? "c" : "d"}
          className="text-[11px] uppercase text-secondary-foreground status-flash"
        >
          {connected ? "connected" : "disconnected"}
        </span>
      </div>

      {/* Config rows */}
      <div className="mt-2 space-y-1.5">
        {/* Model */}
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Model</span>
          <CyberSelect
            value={model}
            options={MODEL_OPTIONS}
            onChange={setModel}
            disabled={connected}
          />
        </div>

        {/* Effort */}
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Effort</span>
          <CyberSelect
            value={effort}
            options={EFFORT_OPTIONS}
            onChange={setEffort}
            disabled={connected}
          />
        </div>

        {/* Project directory */}
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Project</span>
          <button
            type="button"
            onClick={handlePickDir}
            disabled={connected}
            className={cn(
              "inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground transition-colors truncate max-w-44",
              connected
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
      </div>

      {/* Launch button */}
      {!connected && (
        <Button
          size="sm"
          className="w-full mt-2 bg-claude text-white hover:bg-claude/90 hover:shadow-[0_0_16px_#8b5cf640] active:scale-[0.98] transition-all duration-200 btn-ripple"
          disabled={!cwd || connecting}
          onClick={handleLaunch}
        >
          {connecting ? (
            <span className="flex items-center gap-2">
              <span className="size-3 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              Connecting…
            </span>
          ) : (
            "Connect Claude"
          )}
        </Button>
      )}

      {/* Hints */}
      {!connected && !cwd && (
        <div className="mt-1.5 text-[10px] text-muted-foreground text-center">
          Select a project directory first
        </div>
      )}
      {!connected && cwd && !launchError && (
        <div className="mt-1.5 text-[10px] text-muted-foreground text-center">
          Registers .mcp.json and launches Claude in channel preview mode
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
