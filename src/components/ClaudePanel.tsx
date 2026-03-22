import { useEffect, useState, useCallback } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

interface ClaudePanelProps {
  connected: boolean;
}

export function ClaudePanel({ connected }: ClaudePanelProps) {
  const [mcpRegistered, setMcpRegistered] = useState<boolean | null>(null);
  const [registering, setRegistering] = useState(false);

  useEffect(() => {
    invoke<boolean>("check_mcp_registered")
      .then(setMcpRegistered)
      .catch(() => setMcpRegistered(null));
  }, []);

  const handleRegister = useCallback(async () => {
    setRegistering(true);
    try {
      await invoke("register_mcp");
      setMcpRegistered(true);
    } catch {}
    setRegistering(false);
  }, []);

  return (
    <div className="rounded-lg border border-input bg-card p-3">
      <div className="flex items-center gap-2">
        <span
          className={cn(
            "inline-block size-2 shrink-0 rounded-full",
            connected ? "bg-claude" : "bg-muted-foreground",
          )}
        />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Claude Code
        </span>
        <span className="text-[11px] uppercase text-secondary-foreground">
          {connected ? "connected" : "disconnected"}
        </span>
      </div>

      {connected ? (
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
      ) : (
        <div className="mt-2 space-y-2">
          {mcpRegistered === false && (
            <Button
              size="sm"
              className="w-full bg-claude text-white hover:bg-claude/80"
              disabled={registering}
              onClick={handleRegister}
            >
              {registering ? "Registering..." : "Register MCP"}
            </Button>
          )}
          {mcpRegistered === true && (
            <div className="text-[11px] text-muted-foreground leading-relaxed">
              MCP registered. Start Claude Code in this project to connect.
            </div>
          )}
          {mcpRegistered === null && (
            <div className="text-[11px] text-muted-foreground leading-relaxed">
              Add MCP to ~/.claude/mcp.json to connect
            </div>
          )}
        </div>
      )}
    </div>
  );
}
