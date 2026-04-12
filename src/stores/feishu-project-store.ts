import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import * as feishuApi from "./feishu-project-api";

export type McpConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "unauthorized"
  | "error";

export type FeishuSyncMode = "todo" | "issues";

export interface FeishuProjectRuntimeState {
  enabled: boolean;
  domain?: string | null;
  workspaceHint?: string | null;
  refreshIntervalMinutes: number;
  syncMode: FeishuSyncMode;
  projectName?: string | null;
  teamMembers: string[];
  statusOptions: string[];
  assigneeOptions: string[];
  mcpStatus: McpConnectionStatus;
  discoveredToolCount: number;
  lastSyncAt?: number | null;
  lastError?: string | null;
  tokenLabel?: string | null;
}

export interface FeishuProjectInboxItem {
  recordId: string;
  projectKey: string;
  workItemId: string;
  workItemTypeKey: string;
  title: string;
  statusLabel?: string | null;
  assigneeLabel?: string | null;
  updatedAt: number;
  sourceUrl: string;
  rawSnapshotRef: string;
  ignored: boolean;
  linkedTaskId?: string | null;
  lastIngress: "poll" | "webhook" | "mcp";
  lastEventUuid?: string | null;
}

export interface FeishuProjectConfigInput {
  enabled: boolean;
  domain: string;
  mcp_user_token: string;
  workspace_hint: string;
  refresh_interval_minutes: number;
  sync_mode: FeishuSyncMode;
}

export interface IssueFilter {
  status?: string | null;
  assignee?: string | null;
}

interface FeishuProjectStore {
  runtimeState: FeishuProjectRuntimeState | null;
  items: FeishuProjectInboxItem[];
  loading: boolean;
  loadingMore: boolean;
  error: string | null;
  activeFilter: IssueFilter;
  issuesHydrated: boolean;
  fetchState: () => Promise<void>;
  fetchItems: () => Promise<void>;
  hydrateIssuesFirstPage: () => Promise<void>;
  saveConfig: (config: FeishuProjectConfigInput) => Promise<void>;
  syncNow: () => Promise<void>;
  loadMore: () => Promise<void>;
  loadMoreFiltered: () => Promise<void>;
  fetchFilterOptions: () => Promise<void>;
  setFilter: (filter: IssueFilter) => void;
  hasMore: boolean;
  setIgnored: (workItemId: string, ignored: boolean) => Promise<void>;
  startHandling: (workItemId: string) => Promise<void>;
  cleanup: () => void;
}

let unlistenState: UnlistenFn | null = null;
let unlistenItems: UnlistenFn | null = null;

export const useFeishuProjectStore = create<FeishuProjectStore>((set, get) => ({
  runtimeState: null,
  items: [],
  loading: false,
  loadingMore: false,
  error: null,
  hasMore: true,
  activeFilter: {},
  issuesHydrated: false,

  hydrateIssuesFirstPage: async () => {
    set({ issuesHydrated: false, loading: true, error: null });
    try {
      // Backend must have runtime state before filter options can be written
      const rs = await invoke<FeishuProjectRuntimeState>(
        "feishu_project_get_state",
      );
      set({ runtimeState: rs });
      // Fetch filter options into backend state
      await feishuApi.fetchFilterOptions();
      // Re-read state (now with options) + items together
      const [finalRs, items] = await Promise.all([
        invoke<FeishuProjectRuntimeState>("feishu_project_get_state"),
        invoke<FeishuProjectInboxItem[]>("feishu_project_list_items"),
      ]);
      set({ runtimeState: finalRs, items, loading: false, issuesHydrated: true });
    } catch (e) {
      set({ error: String(e), loading: false, issuesHydrated: true });
    }
  },

  fetchState: async () => {
    set({ loading: true, error: null });
    try {
      const rs = await invoke<FeishuProjectRuntimeState>(
        "feishu_project_get_state",
      );
      set({ runtimeState: rs, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  fetchItems: async () => {
    try {
      const items = await invoke<FeishuProjectInboxItem[]>(
        "feishu_project_list_items",
      );
      set({ items });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  saveConfig: async (config) => {
    const prevWorkspace = get().runtimeState?.workspaceHint ?? "";
    set({ loading: true, error: null });
    try {
      const rs = await invoke<FeishuProjectRuntimeState>(
        "feishu_project_save_config",
        { config },
      );
      set({ runtimeState: rs, loading: false });
      // Re-fetch owner/status options only when workspace actually changed
      if ((config.workspace_hint ?? "") !== (prevWorkspace ?? "")) {
        await get().fetchFilterOptions();
      }
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  syncNow: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("feishu_project_sync_now");
      // Fetch items immediately after sync completes
      const items = await invoke<FeishuProjectInboxItem[]>(
        "feishu_project_list_items",
      );
      set({ items, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  loadMore: async () => {
    set({ loadingMore: true, error: null });
    try {
      const count = await invoke<number>("feishu_project_load_more");
      const items = await invoke<FeishuProjectInboxItem[]>(
        "feishu_project_list_items",
      );
      set({ items, loadingMore: false, hasMore: count >= 50 });
    } catch (e) {
      set({ error: String(e), loadingMore: false });
    }
  },

  loadMoreFiltered: async () => {
    const filter = get().activeFilter;
    set({ loadingMore: true, error: null });
    try {
      const count = await feishuApi.loadMoreFiltered(filter);
      const items = await feishuApi.listItems();
      set({ items, loadingMore: false, hasMore: count >= 50 });
    } catch (e) {
      set({ error: String(e), loadingMore: false });
    }
  },

  fetchFilterOptions: async () => {
    try {
      await feishuApi.fetchFilterOptions();
      // Re-read runtime state so statusOptions/assigneeOptions are
      // available immediately, not only after the async event arrives.
      const rs = await invoke<FeishuProjectRuntimeState>(
        "feishu_project_get_state",
      );
      set({ runtimeState: rs });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setFilter: (filter) => {
    set({ activeFilter: filter, items: [], hasMore: true });
  },

  setIgnored: async (workItemId, ignored) => {
    try {
      await invoke("feishu_project_set_ignored", { workItemId, ignored });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  startHandling: async (workItemId) => {
    set({ error: null });
    try {
      await invoke("feishu_project_start_handling", { workItemId });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  cleanup: () => {
    unlistenState?.();
    unlistenState = null;
    unlistenItems?.();
    unlistenItems = null;
  },
}));

// Auto-subscribe to daemon events
void listen<FeishuProjectRuntimeState>("feishu_project_state", (e) => {
  useFeishuProjectStore.setState({ runtimeState: e.payload });
}).then((fn) => {
  unlistenState = fn;
});

void listen<FeishuProjectInboxItem[]>("feishu_project_items", (e) => {
  useFeishuProjectStore.setState({ items: e.payload });
}).then((fn) => {
  unlistenItems = fn;
});
