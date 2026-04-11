import { useCallback, useEffect } from "react";
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
  const saveConfig = useFeishuProjectStore((s) => s.saveConfig);
  const syncNow = useFeishuProjectStore((s) => s.syncNow);
  const loadMoreFiltered = useFeishuProjectStore((s) => s.loadMoreFiltered);
  const fetchFilterOptions = useFeishuProjectStore((s) => s.fetchFilterOptions);
  const activeFilter = useFeishuProjectStore((s) => s.activeFilter);
  const setFilter = useFeishuProjectStore((s) => s.setFilter);
  const hasMore = useFeishuProjectStore((s) => s.hasMore);
  const setIgnored = useFeishuProjectStore((s) => s.setIgnored);
  const startHandling = useFeishuProjectStore((s) => s.startHandling);
  const issuesHydrated = useFeishuProjectStore((s) => s.issuesHydrated);
  const hydrateIssuesFirstPage = useFeishuProjectStore(
    (s) => s.hydrateIssuesFirstPage,
  );

  useEffect(() => {
    void hydrateIssuesFirstPage();
  }, [hydrateIssuesFirstPage]);

  const currentMode: FeishuSyncMode = runtimeState?.syncMode ?? "todo";
  const isConfigured = runtimeState?.tokenLabel || runtimeState?.enabled;

  const handleModeChange = useCallback(
    async (mode: FeishuSyncMode) => {
      setFilter({});
      useFeishuProjectStore.setState({
        items: [],
        loading: true,
        hasMore: true,
        issuesHydrated: false,
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
      await fetchFilterOptions();
      useFeishuProjectStore.setState({ issuesHydrated: true });
    },
    [saveConfig, syncNow, runtimeState, setFilter, fetchFilterOptions],
  );

  const handleAssigneeChange = useCallback(
    async (assignee: string) => {
      const next = { ...activeFilter, assignee: assignee || undefined };
      setFilter(next);
      useFeishuProjectStore.setState({ issuesHydrated: false });
      await loadMoreFiltered();
      useFeishuProjectStore.setState({ issuesHydrated: true });
    },
    [activeFilter, setFilter, loadMoreFiltered],
  );

  const handleStatusChange = useCallback(
    async (status: string) => {
      const next = { ...activeFilter, status: status || undefined };
      setFilter(next);
      useFeishuProjectStore.setState({ issuesHydrated: false });
      await loadMoreFiltered();
      useFeishuProjectStore.setState({ issuesHydrated: true });
    },
    [activeFilter, setFilter, loadMoreFiltered],
  );

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
            assigneeFilter={activeFilter.assignee ?? ""}
            onAssigneeChange={handleAssigneeChange}
            statusOptions={runtimeState?.statusOptions ?? []}
            statusFilter={activeFilter.status ?? ""}
            onStatusChange={handleStatusChange}
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
        {isConfigured && currentMode === "issues" && !issuesHydrated ? (
          <div className="space-y-2 py-2 animate-pulse">
            <div className="h-7 w-36 rounded bg-muted/40" />
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-[52px] rounded-lg bg-muted/20" />
            ))}
          </div>
        ) : (
          <IssueList
            items={items}
            loading={loading}
            loadingMore={loadingMore}
            hasMore={hasMore && currentMode === "issues"}
            onLoadMore={loadMoreFiltered}
            onIgnore={setIgnored}
            onStartHandling={startHandling}
          />
        )}
      </div>
    </section>
  );
}
