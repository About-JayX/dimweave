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
