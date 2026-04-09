import { useCallback, useEffect, useMemo, useState } from "react";
import {
  useFeishuProjectStore,
  type FeishuSyncMode,
} from "@/stores/feishu-project-store";
import { ConfigCard } from "./ConfigCard";
import { SyncModeNav } from "./SyncModeNav";
import { IssueList } from "./IssueList";

export function BugInboxPanel() {
  const runtimeState = useFeishuProjectStore((s) => s.runtimeState);
  const items = useFeishuProjectStore((s) => s.items);
  const loading = useFeishuProjectStore((s) => s.loading);
  const loadingMore = useFeishuProjectStore((s) => s.loadingMore);
  const error = useFeishuProjectStore((s) => s.error);
  const fetchState = useFeishuProjectStore((s) => s.fetchState);
  const fetchItems = useFeishuProjectStore((s) => s.fetchItems);
  const saveConfig = useFeishuProjectStore((s) => s.saveConfig);
  const syncNow = useFeishuProjectStore((s) => s.syncNow);
  const loadMore = useFeishuProjectStore((s) => s.loadMore);
  const hasMore = useFeishuProjectStore((s) => s.hasMore);
  const setIgnored = useFeishuProjectStore((s) => s.setIgnored);
  const startHandling = useFeishuProjectStore((s) => s.startHandling);

  const [assigneeFilter, setAssigneeFilter] = useState("");

  useEffect(() => {
    void fetchState();
    void fetchItems();
  }, [fetchState, fetchItems]);

  const currentMode: FeishuSyncMode = runtimeState?.syncMode ?? "todo";
  const isConfigured = runtimeState?.tokenLabel || runtimeState?.enabled;

  const handleModeChange = useCallback(
    async (mode: FeishuSyncMode) => {
      setAssigneeFilter("");
      useFeishuProjectStore.setState({
        items: [],
        loading: true,
        hasMore: true,
      });
      await saveConfig({
        enabled: true,
        domain: runtimeState?.domain?.trim() || "https://project.feishu.cn",
        mcp_user_token: "",
        workspace_hint: runtimeState?.workspaceHint?.trim() ?? "",
        refresh_interval_minutes: runtimeState?.refreshIntervalMinutes ?? 10,
        sync_mode: mode,
      });
      await syncNow();
    },
    [saveConfig, syncNow, runtimeState],
  );

  const filteredItems = useMemo(() => {
    if (!assigneeFilter) return items;
    return items.filter(
      (i) => i.assigneeLabel && i.assigneeLabel.includes(assigneeFilter),
    );
  }, [items, assigneeFilter]);

  return (
    <section className="flex h-full flex-col -mx-4 -my-4">
      {/* Fixed header area */}
      <div className="shrink-0 space-y-2 px-4 pt-4 pb-2">
        <ConfigCard
          runtimeState={runtimeState}
          loading={loading}
          onSave={saveConfig}
          onSync={syncNow}
        />

        {isConfigured && (
          <SyncModeNav
            value={currentMode}
            onChange={handleModeChange}
            disabled={loading}
            teamMembers={runtimeState?.teamMembers ?? []}
            assigneeFilter={assigneeFilter}
            onAssigneeChange={setAssigneeFilter}
          />
        )}

        {error && (
          <div className="rounded-lg border border-rose-400/30 bg-rose-400/5 px-3 py-1.5 text-[10px] text-rose-400">
            {error}
          </div>
        )}
      </div>

      {/* Scrollable list area */}
      <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        <IssueList
          items={filteredItems}
          loading={loading}
          loadingMore={loadingMore}
          hasMore={hasMore && currentMode === "issues"}
          onLoadMore={loadMore}
          onIgnore={setIgnored}
          onStartHandling={startHandling}
        />
      </div>
    </section>
  );
}
