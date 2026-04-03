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
      description: "Start a fresh session in this workspace",
    },
    ...providerItems.map((entry) => ({
      value: entry.externalId,
      label: entry.title?.trim() || `${provider} session`,
      description:
        entry.preview?.trim() || `Resume ${entry.externalId.slice(0, 24)}`,
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

export function resolveProviderHistoryWorkspace(
  cwd: string,
  session: ProviderSessionInfo | undefined,
): string {
  return cwd || session?.cwd || "";
}
