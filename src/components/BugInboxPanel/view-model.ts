import type { FeishuProjectInboxItem } from "@/stores/feishu-project-store";

export function activeItemCount(items: FeishuProjectInboxItem[]): number {
  return items.filter((i) => !i.ignored).length;
}

export function formatSyncTime(ts: number | null | undefined): string {
  if (!ts) return "Never";
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });
}
