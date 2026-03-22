import { useEffect, useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { useBridgeStore, type TerminalLine } from "@/stores/bridge-store";
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
  const [inputText, setInputText] = useState("");
  const [cwd, setCwd] = useState("");

  const allLines = useBridgeStore((s) => s.terminalLines);
  const launchClaude = useBridgeStore((s) => s.launchClaude);
  const sendClaudeInput = useBridgeStore((s) => s.sendClaudeInput);
  const stopClaude = useBridgeStore((s) => s.stopClaude);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);

  const statusLines: TerminalLine[] = [];
  for (const l of allLines) {
    if (l.agent === "claude") statusLines.push(l);
  }
  const isRunning = statusLines.length > 0;

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
    launchClaude(cwd || undefined);
  }, [mcpRegistered, launchClaude, cwd]);

  const handleSend = useCallback(() => {
    const text = inputText.trim();
    if (!text) return;
    sendClaudeInput(text);
    setInputText("");
  }, [inputText, sendClaudeInput]);

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
        <span className="text-[11px] uppercase text-secondary-foreground">
          {connected ? "connected" : isRunning ? "starting" : "disconnected"}
        </span>
      </div>

      {/* Quota (when connected or running) */}
      {(connected || isRunning) && <ClaudeQuota />}

      {/* Input (when running) */}
      {(isRunning || connected) && (
        <div className="mt-2 flex gap-1.5">
          <input
            type="text"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Send to Claude..."
            className="flex-1 rounded-md border border-input bg-background px-2 py-1 text-[11px] text-foreground outline-none placeholder:text-muted-foreground focus:border-ring"
          />
          <Button size="xs" variant="secondary" onClick={handleSend}>
            Send
          </Button>
          <Button size="xs" variant="destructive" onClick={stopClaude}>
            Stop
          </Button>
        </div>
      )}

      {/* Launch (when not running) */}
      {!isRunning && !connected && (
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
              {cwd ? shortenPath(cwd) : "Select..."}
            </button>
          </div>
          <Button
            size="sm"
            className="w-full bg-claude text-white hover:bg-claude/80"
            onClick={handleLaunch}
          >
            Connect Claude
          </Button>
        </div>
      )}
    </div>
  );
}
