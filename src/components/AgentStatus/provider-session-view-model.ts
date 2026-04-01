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

export function formatProviderConnectionLabel(
  session: ProviderSessionInfo | undefined,
): string | null {
  if (!session) return null;
  const shortId = session.externalSessionId.slice(0, 24);
  if (session.provider === "codex") {
    return session.connectionMode === "resumed"
      ? `Resumed thread ${shortId}`
      : `New thread ${shortId}`;
  }
  return session.connectionMode === "resumed"
    ? `Resumed ${shortId}`
    : `New session ${shortId}`;
}

export function resolveProviderHistoryWorkspace(
  cwd: string,
  session: ProviderSessionInfo | undefined,
): string {
  return cwd || session?.cwd || "";
}
