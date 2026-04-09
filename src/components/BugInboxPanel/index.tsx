import { useEffect } from "react";
import { useFeishuProjectStore } from "@/stores/feishu-project-store";
import { ConfigCard } from "./ConfigCard";
import { IssueList } from "./IssueList";

export function BugInboxPanel() {
  const runtimeState = useFeishuProjectStore((s) => s.runtimeState);
  const items = useFeishuProjectStore((s) => s.items);
  const loading = useFeishuProjectStore((s) => s.loading);
  const error = useFeishuProjectStore((s) => s.error);
  const fetchState = useFeishuProjectStore((s) => s.fetchState);
  const fetchItems = useFeishuProjectStore((s) => s.fetchItems);
  const saveConfig = useFeishuProjectStore((s) => s.saveConfig);
  const syncNow = useFeishuProjectStore((s) => s.syncNow);
  const setIgnored = useFeishuProjectStore((s) => s.setIgnored);
  const startHandling = useFeishuProjectStore((s) => s.startHandling);

  useEffect(() => {
    void fetchState();
    void fetchItems();
  }, [fetchState, fetchItems]);

  return (
    <section className="space-y-3">
      <ConfigCard
        runtimeState={runtimeState}
        loading={loading}
        onSave={saveConfig}
        onSync={syncNow}
      />

      {error && (
        <div className="rounded-lg border border-rose-400/30 bg-rose-400/5 px-3 py-1.5 text-[10px] text-rose-400">
          {error}
        </div>
      )}

      <IssueList
        items={items}
        onIgnore={setIgnored}
        onStartHandling={startHandling}
      />
    </section>
  );
}
