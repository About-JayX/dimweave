interface BuildClaudeLaunchRequestInput {
  claudeRole: string;
  cwd: string;
  model?: string | null;
  effort?: string | null;
  resumeSessionId?: string | null;
  taskId?: string | null;
}

interface ClaudeLaunchRequest {
  roleId: string;
  cwd: string;
  model: string | null;
  effort: string | null;
  resumeSessionId: string | null;
  taskId: string | null;
}

function normalizeOptional(value?: string | null): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

function normalizeRequiredRole(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error("Select Claude role before connecting");
  }
  return trimmed;
}

export function buildClaudeLaunchRequest(
  input: BuildClaudeLaunchRequestInput,
): ClaudeLaunchRequest {
  return {
    roleId: normalizeRequiredRole(input.claudeRole),
    cwd: input.cwd,
    model: normalizeOptional(input.model),
    effort: normalizeOptional(input.effort),
    resumeSessionId: normalizeOptional(input.resumeSessionId),
    taskId: input.taskId ?? null,
  };
}
