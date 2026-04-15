import type { CyberSelectOption } from "@/components/ui/cyber-select";
import type { ProviderHistoryInfo } from "@/stores/task-store/types";
import type { ProviderSessionInfo } from "@/types";

export const NEW_PROVIDER_SESSION_VALUE = "__new_session__";

export function buildProviderHistoryOptions(
  provider: ProviderHistoryInfo["provider"],
  history: ProviderHistoryInfo[],
): CyberSelectOption[] {
  const providerItems = history
    .filter((entry) => entry.provider === provider)
    .sort((a, b) => b.updatedAt - a.updatedAt);

  return [
    {
      value: NEW_PROVIDER_SESSION_VALUE,
      label: "New session",
    },
    ...providerItems.map((entry) => ({
      value: entry.externalId,
      label:
        entry.title?.trim() || entry.preview?.trim() || `${provider} session`,
      description: formatHistoryMeta(entry),
    })),
  ];
}

export function findProviderHistoryEntry(
  provider: ProviderHistoryInfo["provider"],
  history: ProviderHistoryInfo[],
  selectedValue: string,
): ProviderHistoryInfo | null {
  if (selectedValue === NEW_PROVIDER_SESSION_VALUE) {
    return null;
  }
  return (
    history.find(
      (entry) =>
        entry.provider === provider && entry.externalId === selectedValue,
    ) ?? null
  );
}

export interface ConnectionLabel {
  short: string;
  full: string;
}

function truncateId(id: string): string {
  if (id.length <= 12) return id;
  return `${id.slice(0, 6)}…${id.slice(-4)}`;
}

function formatHistoryMeta(entry: ProviderHistoryInfo): string {
  const date = new Date(entry.updatedAt).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
  return `${truncateId(entry.externalId)} · ${date}`;
}

export function formatProviderConnectionLabel(
  session: ProviderSessionInfo | undefined,
): ConnectionLabel | null {
  if (!session) return null;
  const id = session.externalSessionId;
  const shortId = truncateId(id);
  const mode = session.connectionMode === "resumed" ? "Resumed" : "New";
  const kind = session.provider === "codex" ? "thread" : "session";
  return { short: `${mode} ${kind} ${shortId}`, full: `${mode} ${kind} ${id}` };
}

export type ProviderHistoryAction =
  | { kind: "new" }
  | { kind: "resumeNormalized"; sessionId: string }
  | { kind: "resumeExternal"; externalId: string };

export interface AgentDraftConfig {
  model: string;
  effort: string;
  historyAction: ProviderHistoryAction;
}

export function resolveProviderHistoryAction(
  entry: Pick<ProviderHistoryInfo, "externalId" | "normalizedSessionId"> | null,
): ProviderHistoryAction {
  if (!entry) return { kind: "new" };
  if (entry.normalizedSessionId) {
    return { kind: "resumeNormalized", sessionId: entry.normalizedSessionId };
  }
  return { kind: "resumeExternal", externalId: entry.externalId };
}

export function resolveProviderHistoryWorkspace(
  activeTaskWorkspace: string | null | undefined,
): string {
  return activeTaskWorkspace?.trim() || "";
}

// --- Provider capability flags (shared with TaskSetupDialog) ---

export interface SelectOption { value: string; label: string }

export interface ProviderCaps {
  supportsModel: boolean;
  supportsEffort: boolean;
  effortRequiresModel: boolean;
  supportsSessionResume: boolean;
  effortLabel: string;
  resumeIdPlaceholder: string;
  modelOptions: SelectOption[];
  effortOptions: SelectOption[];
}

export const PROVIDER_CAPS: Record<"claude" | "codex", ProviderCaps> = {
  claude: {
    supportsModel: true,
    supportsEffort: true,
    effortRequiresModel: false,
    supportsSessionResume: true,
    effortLabel: "Effort",
    resumeIdPlaceholder: "session ID",
    modelOptions: [
      { value: "claude-sonnet-4-5-20250514", label: "Sonnet 4.5" },
      { value: "claude-opus-4-5-20250514", label: "Opus 4.5" },
      { value: "claude-haiku-3-5-20241022", label: "Haiku 3.5" },
    ],
    effortOptions: [
      { value: "low", label: "Low" },
      { value: "medium", label: "Medium" },
      { value: "high", label: "High" },
    ],
  },
  codex: {
    supportsModel: true,
    supportsEffort: true,
    effortRequiresModel: true,
    supportsSessionResume: true,
    effortLabel: "Reasoning effort",
    resumeIdPlaceholder: "thread ID",
    modelOptions: [
      { value: "o3-pro", label: "o3-pro" },
      { value: "o3", label: "o3" },
      { value: "o4-mini", label: "o4-mini" },
    ],
    effortOptions: [
      { value: "low", label: "Low" },
      { value: "medium", label: "Medium" },
      { value: "high", label: "High" },
    ],
  },
};

// --- Agent config form helpers (shared with TaskSetupDialog) ---

export function deriveSessionMode(ha?: ProviderHistoryAction): "new" | "resume" {
  if (!ha || ha.kind === "new") return "new";
  return "resume";
}

export function deriveResumeId(ha?: ProviderHistoryAction): string {
  if (ha?.kind === "resumeExternal") return ha.externalId;
  if (ha?.kind === "resumeNormalized") return ha.sessionId;
  return "";
}

export function buildHistoryAction(mode: "new" | "resume", resumeId: string): ProviderHistoryAction {
  if (mode === "resume" && resumeId.trim()) {
    return { kind: "resumeExternal", externalId: resumeId.trim() };
  }
  return { kind: "new" };
}

export function buildDraftConfigFromDef(def: {
  model?: string; effort?: string; historyAction?: ProviderHistoryAction;
}): AgentDraftConfig {
  return {
    model: def.model ?? "",
    effort: def.effort ?? "",
    historyAction: def.historyAction ?? { kind: "new" },
  };
}

export function historyActionToSelectValue(
  ha: ProviderHistoryAction | undefined,
  history?: ProviderHistoryInfo[],
): string {
  if (!ha || ha.kind === "new") return NEW_PROVIDER_SESSION_VALUE;
  if (ha.kind === "resumeExternal") return ha.externalId;
  if (ha.kind === "resumeNormalized" && history) {
    const entry = history.find((e) => e.normalizedSessionId === ha.sessionId);
    if (entry) return entry.externalId;
  }
  return NEW_PROVIDER_SESSION_VALUE;
}
