import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ProviderAuthConfig {
  provider: "claude" | "codex";
  apiKey?: string | null;
  baseUrl?: string | null;
  wireApi?: string | null;
  authMode?: string | null;
  providerName?: string | null;
  updatedAt: number;
}

interface ProviderAuthState {
  configs: Partial<Record<"claude" | "codex", ProviderAuthConfig>>;
  loading: boolean;
  saveError: string | null;
  fetchAll: () => Promise<void>;
  save: (config: ProviderAuthConfig) => Promise<boolean>;
  clear: (provider: "claude" | "codex") => Promise<void>;
}

export const useProviderAuthStore = create<ProviderAuthState>((set, get) => ({
  configs: {},
  loading: false,
  saveError: null,

  fetchAll: async () => {
    set({ loading: true });
    const next: ProviderAuthState["configs"] = {};
    for (const provider of ["claude", "codex"] as const) {
      try {
        const cfg = await invoke<ProviderAuthConfig | null>(
          "daemon_get_provider_auth",
          { provider },
        );
        if (cfg) next[provider] = cfg;
      } catch (e) {
        console.error("[ProviderAuth] fetchAll", provider, e);
      }
    }
    set({ configs: next, loading: false });
  },

  save: async (config) => {
    set({ saveError: null });
    try {
      await invoke("daemon_save_provider_auth", { config });
      await get().fetchAll();
      return true;
    } catch (e) {
      const msg = String(e);
      console.error("[ProviderAuth] save", msg);
      set({ saveError: msg });
      return false;
    }
  },

  clear: async (provider) => {
    try {
      await invoke("daemon_clear_provider_auth", { provider });
      const { [provider]: _removed, ...rest } = get().configs;
      set({ configs: rest });
    } catch (e) {
      console.error("[ProviderAuth] clear", provider, e);
    }
  },
}));
