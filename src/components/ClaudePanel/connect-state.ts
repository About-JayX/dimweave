interface ClaudeConnectState {
  cwd: string;
  role: string;
  connecting: boolean;
  connected: boolean;
  disconnecting: boolean;
}

export function canConnectClaude({
  cwd,
  role,
  connecting,
  connected,
  disconnecting,
}: ClaudeConnectState): boolean {
  return (
    cwd.trim().length > 0 &&
    role.trim().length > 0 &&
    !connecting &&
    !connected &&
    !disconnecting
  );
}
