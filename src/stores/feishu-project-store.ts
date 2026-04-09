import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";

export interface FeishuProjectRuntimeState {
  enabled: boolean;
  projectKey?: string | null;
  tokenLabel?: string | null;
  userKey?: string | null;
  pollIntervalMinutes: number;
  publicWebhookBaseUrl?: string | null;
  localWebhookPath: string;
  lastPollAt?: number | null;
  lastWebhookAt?: number | null;
  lastSyncAt?: number | null;
  lastError?: string | null;
  webhookEnabled: boolean;
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
  lastIngress: "poll" | "webhook";
  lastEventUuid?: string | null;
}

export interface FeishuProjectConfigInput {
  enabled: boolean;
  project_key: string;
  plugin_token: string;
  user_key: string;
  webhook_token: string;
  poll_interval_minutes: number;
  public_webhook_base_url?: string | null;
}

interface FeishuProjectStore {
  runtimeState: FeishuProjectRuntimeState | null;
  items: FeishuProjectInboxItem[];
  loading: boolean;
  error: string | null;
  fetchState: () => Promise<void>;
  fetchItems: () => Promise<void>;
  saveConfig: (config: FeishuProjectConfigInput) => Promise<void>;
  syncNow: () => Promise<void>;
  setIgnored: (workItemId: string, ignored: boolean) => Promise<void>;
  startHandling: (workItemId: string) => Promise<void>;
  cleanup: () => void;
}

let unlistenState: UnlistenFn | null = null;
let unlistenItems: UnlistenFn | null = null;

export const useFeishuProjectStore = create<FeishuProjectStore>((set) => ({
  runtimeState: null,
  items: [],
  loading: false,
  error: null,

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
    set({ loading: true, error: null });
    try {
      const rs = await invoke<FeishuProjectRuntimeState>(
        "feishu_project_save_config",
        { config },
      );
      set({ runtimeState: rs, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  syncNow: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("feishu_project_sync_now");
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
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
