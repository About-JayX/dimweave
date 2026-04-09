interface ReasoningModelLike {
  defaultReasoningLevel: string | null;
  reasoningLevels: { effort: string }[];
}

interface ConnectState {
  cwd: string;
  role: string;
  connecting: boolean;
  running: boolean;
}

interface CodexLaunchInputs {
  model?: string;
  reasoningEffort?: string;
  cwd?: string;
  resumeThreadId?: string;
}

interface CodexConnectTimeoutState {
  connecting: boolean;
  running: boolean;
  connectStartedAt: number | null;
  now?: number;
}

export const CODEX_CONNECT_READY_TIMEOUT_MS = 8_000;

export function getDefaultReasoningEffort(
  model: ReasoningModelLike | undefined,
): string {
  if (!model) {
    return "";
  }
  return model.defaultReasoningLevel || model.reasoningLevels[0]?.effort || "";
}

export function canConnectCodex({
  cwd,
  role,
  connecting,
  running,
}: ConnectState): boolean {
  return (
    cwd.trim().length > 0 &&
    role.trim().length > 0 &&
    !connecting &&
    !running
  );
}

export function buildCodexLaunchConfig({
  model,
  reasoningEffort,
  cwd,
  resumeThreadId,
}: CodexLaunchInputs): {
  model?: string;
  reasoningEffort?: string;
  cwd?: string;
  resumeThreadId?: string;
} {
  return {
    model: model || undefined,
    reasoningEffort: reasoningEffort || undefined,
    cwd: cwd?.trim() || undefined,
    resumeThreadId: resumeThreadId?.trim() || undefined,
  };
}

export function hasCodexConnectTimedOut({
  connecting,
  running,
  connectStartedAt,
  now = Date.now(),
}: CodexConnectTimeoutState): boolean {
  return (
    connecting &&
    !running &&
    connectStartedAt !== null &&
    now - connectStartedAt >= CODEX_CONNECT_READY_TIMEOUT_MS
  );
}

export function getCodexConnectTimeoutMessage(): string {
  return "Codex launch did not report ready state. Check diagnostics and retry.";
}
