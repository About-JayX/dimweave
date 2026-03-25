const cls = "mt-1.5 text-center text-[10px]";

interface ClaudeHintProps {
  connected: boolean;
  cwd: string;
  terminalRunning: boolean;
  disconnecting: boolean;
  actionError: string | null;
}

export function ClaudeHint({
  connected,
  cwd,
  terminalRunning,
  disconnecting,
  actionError,
}: ClaudeHintProps) {
  if (actionError)
    return <div className={`${cls} text-destructive`}>{actionError}</div>;
  if (!connected && !cwd)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Select a project directory first
      </div>
    );
  if (!connected && terminalRunning)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Claude terminal running, waiting for channel startup
      </div>
    );
  if (!connected && cwd)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Registers .mcp.json and launches Claude in channel preview mode
      </div>
    );
  if (connected && disconnecting)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Waiting for Claude terminal to exit
      </div>
    );
  return null;
}
