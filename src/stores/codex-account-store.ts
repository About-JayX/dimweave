import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface CodexProfile {
  email?: string;
  name?: string;
  planType?: string;
  accountId?: string;
  userId?: string;
  orgTitle?: string;
  subscriptionActiveUntil?: string;
}

interface UsageWindow {
  usedPercent: number;
  remainingPercent: number;
  windowMinutes: number | null;
  resetAt: number | null;
  resetAfterSeconds: number | null;
}

interface UsageSnapshot {
  source: string;
  checkedAt: string;
  allowed: boolean;
  limitReached: boolean;
  primary: UsageWindow | null;
  secondary: UsageWindow | null;
}

interface ReasoningLevel {
  effort: string;
  description: string;
}

interface CodexModel {
  slug: string;
  displayName: string;
  defaultReasoningLevel: string | null;
  reasoningLevels: ReasoningLevel[];
}

interface OAuthLaunchInfo {
  verificationUri: string | null;
}

interface CodexAccountState {
  profile: CodexProfile | null;
  usage: UsageSnapshot | null;
  models: CodexModel[];
  loading: boolean;
  refreshing: boolean;
  loginPending: boolean;
  loginUri: string | null;
  apiKeyLoginPending: boolean;
  apiKeyLoginError: string | null;

  fetchProfile: () => Promise<void>;
  fetchUsage: () => Promise<void>;
  refreshUsage: () => Promise<void>;
  fetchModels: () => Promise<void>;
  pickDirectory: () => Promise<string | null>;
  login: () => Promise<void>;
  loginWithApiKey: (apiKey: string) => Promise<boolean>;
  cancelLogin: () => Promise<void>;
  logout: () => Promise<void>;
}

export type { CodexProfile, UsageSnapshot, CodexModel, ReasoningLevel };

let _loginPollInterval: ReturnType<typeof setInterval> | null = null;
let _loginPollTimeout: ReturnType<typeof setTimeout> | null = null;

function clearLoginPolling() {
  if (_loginPollInterval) {
    clearInterval(_loginPollInterval);
    _loginPollInterval = null;
  }
  if (_loginPollTimeout) {
    clearTimeout(_loginPollTimeout);
    _loginPollTimeout = null;
  }
}

export const useCodexAccountStore = create<CodexAccountState>((set, get) => ({
  profile: null,
  usage: null,
  models: [],
  loading: false,
  refreshing: false,
  loginPending: false,
  loginUri: null,
  apiKeyLoginPending: false,
  apiKeyLoginError: null,

  fetchProfile: async () => {
    try {
      const profile = await invoke<CodexProfile>("get_codex_account");
      set({ profile });
    } catch (e) {
      console.error("[CodexAccount]", e);
    }
  },

  fetchUsage: async () => {
    if (get().loading) return;
    set({ loading: true });
    try {
      const usage = await invoke<UsageSnapshot>("refresh_usage");
      set({ usage, loading: false });
    } catch (e) {
      console.error("[CodexAccount]", e);
      set({ loading: false });
    }
  },

  refreshUsage: async () => {
    set({ refreshing: true });
    try {
      const usage = await invoke<UsageSnapshot>("refresh_usage");
      set({ usage, refreshing: false });
    } catch (e) {
      console.error("[CodexAccount]", e);
      set({ refreshing: false });
    }
  },

  fetchModels: async () => {
    try {
      const models = await invoke<CodexModel[]>("list_codex_models");
      set({ models });
    } catch (e) {
      console.error("[CodexAccount]", e);
    }
  },

  pickDirectory: async () => {
    try {
      return await invoke<string | null>("pick_directory");
    } catch (e) {
      console.error("[CodexAccount]", e);
      return null;
    }
  },

  login: async () => {
    set({ loginPending: true, loginUri: null });
    try {
      const info = await invoke<OAuthLaunchInfo>("codex_login");
      if (!info.verificationUri) {
        set({ loginPending: false, loginUri: null });
        console.error(
          "[CodexAccount] login returned no verification URI — possible rate limit",
        );
        return;
      }
      set({ loginUri: info.verificationUri });
      // Poll for auth completion — stop when profile has email
      clearLoginPolling();
      _loginPollInterval = setInterval(async () => {
        try {
          const profile = await invoke<CodexProfile>("get_codex_account");
          if (profile?.email) {
            clearLoginPolling();
            set({ profile, loginPending: false, loginUri: null });
            // Refresh dependent data
            get().fetchUsage();
            get().fetchModels();
          }
        } catch (e) {
          console.error("[CodexAccount]", e);
        }
      }, 2000);
      // Safety timeout
      _loginPollTimeout = setTimeout(() => {
        clearLoginPolling();
        if (get().loginPending) set({ loginPending: false });
      }, 120000);
    } catch (e) {
      console.error("[CodexAccount]", e);
      set({ loginPending: false, loginUri: null });
    }
  },

  loginWithApiKey: async (apiKey) => {
    const trimmed = apiKey.trim();
    if (!trimmed) {
      set({ apiKeyLoginError: "API key is empty" });
      return false;
    }
    set({ apiKeyLoginPending: true, apiKeyLoginError: null });
    try {
      await invoke("codex_login_with_api_key", { apiKey: trimmed });
      await get().fetchProfile();
      set({ apiKeyLoginPending: false, apiKeyLoginError: null });
      // Best-effort refresh for downstream UI
      void get().fetchModels();
      return true;
    } catch (e) {
      const msg = String(e);
      console.error("[CodexAccount] loginWithApiKey", msg);
      set({ apiKeyLoginPending: false, apiKeyLoginError: msg });
      return false;
    }
  },

  cancelLogin: async () => {
    clearLoginPolling();
    try {
      await invoke<boolean>("codex_cancel_login");
    } catch (e) {
      console.error("[CodexAccount]", e);
    }
    set({ loginPending: false, loginUri: null });
  },

  logout: async () => {
    try {
      await invoke("codex_logout");
      set({ profile: null, usage: null, models: [] });
    } catch (e) {
      console.error("[CodexAccount]", e);
    }
  },
}));
