import { useEffect, useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { useBridgeStore } from "@/stores/bridge-store";

const ROLE_OPTIONS = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
];

function ClaudeRoleSelect({ disabled }: { disabled: boolean }) {
  const role = useBridgeStore((s) => s.claudeRole);
  const setRole = useBridgeStore((s) => s.setAgentRole);
  return (
    <select
      value={role}
      onChange={(e) => setRole("claude", e.target.value)}
      disabled={disabled}
      className={cn(
        "rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-foreground border border-input outline-none",
        disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
      )}
    >
      {ROLE_OPTIONS.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
import { useCodexAccountStore } from "@/stores/codex-account-store";

function shortenPath(p: string): string {
  const idx = p.indexOf("/Users/");
  if (idx >= 0) {
    const rest = p.slice(idx + 7);
    const slash = rest.indexOf("/");
    return slash >= 0 ? `~${rest.slice(slash)}` : "~";
  }
  return p;
}

function formatTimeLeft(resetsAt: number): string {
  const secs = Math.max(0, resetsAt - Date.now() / 1000);
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

function barColor(status: string) {
  return status === "allowed" ? "bg-claude" : "bg-destructive";
}

function ClaudeQuota() {
  const rl = useBridgeStore((s) => s.claudeRateLimit);
  if (!rl) return null;

  const label = rl.rateLimitType === "five_hour" ? "5h" : rl.rateLimitType;
  const timeLeft = formatTimeLeft(rl.resetsAt);
  const isAllowed = rl.status === "allowed";

  // Estimate window progress from time (5h = 18000s window)
  const windowSecs = rl.rateLimitType === "five_hour" ? 18000 : 604800;
  const elapsed = windowSecs - Math.max(0, rl.resetsAt - Date.now() / 1000);
  const windowPercent = Math.min(
    100,
    Math.max(0, (elapsed / windowSecs) * 100),
  );

  return (
    <div className="mt-2 rounded-md bg-muted/40 px-3 py-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase text-muted-foreground">
          Claude 额度
        </span>
        <span
          className={cn(
            "rounded-full px-1.5 py-px text-[9px] font-semibold",
            isAllowed
              ? "bg-claude/10 text-claude"
              : "bg-destructive/10 text-destructive",
          )}
        >
          {isAllowed ? "正常" : "受限"}
        </span>
      </div>

      <div>
        <div className="flex items-center justify-between text-[10px] mb-1">
          <span className="text-muted-foreground">{label} window</span>
          <span className="font-mono text-muted-foreground">
            resets {timeLeft}
          </span>
        </div>
        <div className="h-1.5 rounded-full bg-secondary overflow-hidden">
          <div
            className={cn(
              "h-full rounded-full transition-all",
              barColor(rl.status),
            )}
            style={{ width: `${windowPercent}%` }}
          />
        </div>
      </div>
    </div>
  );
}

interface ClaudePanelProps {
  connected: boolean;
}

export function ClaudePanel({ connected }: ClaudePanelProps) {
  const [mcpRegistered, setMcpRegistered] = useState<boolean | null>(null);
  const [cwd, setCwd] = useState("");

  const isRunning = useBridgeStore((s) => s.claudePtyRunning);
  const launchClaude = useBridgeStore((s) => s.launchClaude);
  const stopClaude = useBridgeStore((s) => s.stopClaude);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);

  useEffect(() => {
    invoke<boolean>("check_mcp_registered")
      .then(setMcpRegistered)
      .catch(() => {});
  }, []);

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const handleLaunch = useCallback(async () => {
    if (!mcpRegistered) {
      try {
        await invoke("register_mcp");
        setMcpRegistered(true);
      } catch {}
    }
    if (!cwd) return;
    launchClaude(cwd);
    window.dispatchEvent(new CustomEvent("switch-to-terminal"));
  }, [mcpRegistered, launchClaude, cwd]);

  return (
    <div className="rounded-lg border border-input bg-card p-3">
      {/* Header */}
      <div className="flex items-center gap-2">
        <span
          className={cn(
            "inline-block size-2 shrink-0 rounded-full",
            connected
              ? "bg-claude"
              : isRunning
                ? "bg-yellow-500 animate-pulse"
                : "bg-muted-foreground",
          )}
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Claude Code
        </span>
        <ClaudeRoleSelect disabled={connected || isRunning} />
        {(connected || isRunning) && (
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
        <span className="text-[11px] uppercase text-secondary-foreground">
          {connected ? "connected" : isRunning ? "starting" : "disconnected"}
        </span>
      </div>

      {/* Quota (when connected or running) */}
      {(connected || isRunning) && <ClaudeQuota />}

      {/* Stop button (when running) */}
      {(isRunning || connected) && (
        <Button
          size="xs"
          variant="destructive"
          className="w-full mt-2"
          onClick={stopClaude}
        >
          Stop Claude
        </Button>
      )}

      {/* Launch (when not running) */}
      {!isRunning && (
        <div className="mt-2 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Project</span>
            <button
              type="button"
              onClick={handlePickDir}
              className="inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground hover:bg-accent hover:text-primary transition-colors truncate max-w-44"
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
          <Button
            size="sm"
            className="w-full bg-claude text-white hover:bg-claude/80"
            disabled={!cwd}
            onClick={handleLaunch}
          >
            Connect Claude
          </Button>
          {!cwd && (
            <div className="text-[10px] text-muted-foreground text-center">
              Please select a project directory first
            </div>
          )}
        </div>
      )}
    </div>
  );
}
