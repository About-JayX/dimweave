import { useEffect, useState, useCallback, useRef } from "react";
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

function kindStyle(kind: string): string {
  switch (kind) {
    case "text":
      return "text-foreground/80 font-mono";
    case "tool_use":
      return "text-yellow-500/90 font-mono";
    case "tool_result":
      return "text-muted-foreground font-mono";
    case "status":
      return "text-blue-400";
    case "error":
      return "text-destructive";
    case "cost":
      return "text-muted-foreground";
    default:
      return "text-foreground/60 font-mono";
  }
}

interface ClaudePanelProps {
  connected: boolean;
}

export function ClaudePanel({ connected }: ClaudePanelProps) {
  const [mcpRegistered, setMcpRegistered] = useState<boolean | null>(null);
  const [inputText, setInputText] = useState("");
  const [cwd, setCwd] = useState("");
  const [terminalExpanded, setTerminalExpanded] = useState(true);
  const terminalRef = useRef<HTMLDivElement>(null);

  const allLines = useBridgeStore((s) => s.terminalLines);
  const launchClaude = useBridgeStore((s) => s.launchClaude);
  const sendClaudeInput = useBridgeStore((s) => s.sendClaudeInput);
  const stopClaude = useBridgeStore((s) => s.stopClaude);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);

  const claudeLines: TerminalLine[] = [];
  for (const l of allLines) {
    if (l.agent === "claude") claudeLines.push(l);
  }
  const hasTerminal = claudeLines.length > 0;

  useEffect(() => {
    invoke<boolean>("check_mcp_registered")
      .then(setMcpRegistered)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [allLines]);

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
              : hasTerminal
                ? "bg-yellow-500 animate-pulse"
                : "bg-muted-foreground",
          )}
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Claude Code
        </span>
        <span className="text-[11px] uppercase text-secondary-foreground">
          {connected ? "connected" : hasTerminal ? "starting" : "disconnected"}
        </span>
      </div>

      {/* Project path + Launch (when not running) */}
      {!hasTerminal && !connected && (
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

      {/* Terminal output (collapsible) */}
      {hasTerminal && (
        <>
          <button
            type="button"
            onClick={() => setTerminalExpanded(!terminalExpanded)}
            className="mt-2 flex w-full items-center justify-between text-[10px] text-muted-foreground hover:text-foreground transition-colors"
          >
            <span>Terminal ({claudeLines.length} lines)</span>
            <svg
              width="8"
              height="8"
              viewBox="0 0 12 12"
              className={cn(
                "transition-transform duration-150",
                !terminalExpanded && "-rotate-90",
              )}
            >
              <path
                d="M3 5l3 3 3-3"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              />
            </svg>
          </button>

          {terminalExpanded && (
            <div
              ref={terminalRef}
              className="mt-1 max-h-40 overflow-y-auto rounded-md bg-background p-2 text-[11px] leading-relaxed"
            >
              {claudeLines.map((l, i) => (
                <div
                  key={i}
                  className={cn(
                    "whitespace-pre-wrap py-0.5",
                    kindStyle(l.kind),
                  )}
                >
                  {l.kind === "tool_use" && (
                    <span className="text-yellow-500 mr-1">⚡</span>
                  )}
                  {l.kind === "tool_result" && (
                    <span className="text-muted-foreground mr-1">→</span>
                  )}
                  {l.kind === "status" && (
                    <span className="text-blue-400 mr-1">●</span>
                  )}
                  {l.kind === "error" && (
                    <span className="text-destructive mr-1">✕</span>
                  )}
                  {l.kind === "cost" && (
                    <span className="text-muted-foreground mr-1">$</span>
                  )}
                  {l.line}
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {/* Input + controls (when running) */}
      {hasTerminal && (
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

      {/* Connected info */}
      {connected && !hasTerminal && (
        <div className="mt-2 rounded-md bg-muted/40 px-3 py-2 space-y-1">
          <div className="flex items-center justify-between text-[11px]">
            <span className="text-muted-foreground">MCP</span>
            <span className="font-medium text-codex">registered</span>
          </div>
          <div className="flex items-center justify-between text-[11px]">
            <span className="text-muted-foreground">Tools</span>
            <span className="font-mono text-secondary-foreground">
              reply · check · status
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
