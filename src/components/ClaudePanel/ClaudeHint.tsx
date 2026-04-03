const cls = "mt-1.5 text-center text-[10px]";

interface ClaudeHintProps {
  connected: boolean;
  cwd: string;
  disconnecting: boolean;
  actionError: string | null;
}

export function ClaudeHint({
  connected,
  cwd,
  disconnecting,
  actionError,
}: ClaudeHintProps) {
  if (actionError)
    return <div className={`${cls} text-destructive`}>{actionError}</div>;
  if (!connected && !cwd)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Select a workspace from the shell first
      </div>
    );
  if (!connected && cwd)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Launches Claude via --sdk-url with workspace MCP config
      </div>
    );
  if (connected && disconnecting)
    return (
      <div className={`${cls} text-muted-foreground`}>
        Disconnecting Claude SDK session
      </div>
    );
  return null;
}
