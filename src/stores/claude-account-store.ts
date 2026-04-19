import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ClaudeModel {
  slug: string;
  displayName: string;
  supportedEfforts: string[];
}

export interface ClaudeProfile {
  email: string;
  displayName: string;
  subscriptionTier: string;
  rateLimitTier: string;
  organizationName: string;
  subscriptionStatus: string;
}

export interface ClaudeUsageWindow {
  utilization: number;
  resetsAt: number | null;
  status: string;
}

export interface ClaudeUsage {
  overallStatus: string;
  fiveHour: ClaudeUsageWindow | null;
  sevenDay: ClaudeUsageWindow | null;
}

export interface ClaudeAuthStatus {
  loggedIn: boolean;
  authMethod?: string | null;
  apiProvider?: string | null;
  email?: string | null;
  orgId?: string | null;
  orgName?: string | null;
  subscriptionType?: string | null;
}

interface ClaudeAccountState {
  models: ClaudeModel[];
  profile: ClaudeProfile | null;
  profileError: string | null;
  usage: ClaudeUsage | null;
  usageError: string | null;
  usageRefreshing: boolean;
  fetchFailed: boolean;
  authStatus: ClaudeAuthStatus | null;
  loginPending: boolean;
  loginUri: string | null;
  loginError: string | null;
  fetchModels: () => Promise<void>;
  fetchProfile: () => Promise<void>;
  refreshUsage: () => Promise<void>;
  fetchAuthStatus: () => Promise<void>;
  login: () => Promise<void>;
  cancelLogin: () => Promise<void>;
  logout: () => Promise<void>;
}

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

export const useClaudeAccountStore = create<ClaudeAccountState>((set, get) => ({
  models: [],
  profile: null,
  profileError: null,
  usage: null,
  usageError: null,
  usageRefreshing: false,
  fetchFailed: false,
  authStatus: null,
  loginPending: false,
  loginUri: null,
  loginError: null,
  fetchModels: async () => {
    try {
      const models = await invoke<ClaudeModel[]>("list_claude_models");
      set({ models, fetchFailed: false });
    } catch (e) {
      console.error("[ClaudeAccount]", e);
      set({ fetchFailed: true });
    }
  },
  fetchProfile: async () => {
    try {
      const profile = await invoke<ClaudeProfile>("get_claude_profile");
      set({ profile, profileError: null });
    } catch (e) {
      console.error("[ClaudeAccount] profile", e);
      set({ profileError: String(e) });
    }
  },
  refreshUsage: async () => {
    if (get().usageRefreshing) return;
    set({ usageRefreshing: true });
    try {
      const usage = await invoke<ClaudeUsage>("get_claude_usage");
      set({ usage, usageError: null, usageRefreshing: false });
    } catch (e) {
      console.error("[ClaudeAccount] usage", e);
      set({ usageError: String(e), usageRefreshing: false });
    }
  },

  fetchAuthStatus: async () => {
    try {
      const status = await invoke<ClaudeAuthStatus>("claude_auth_status");
      set({ authStatus: status });
    } catch (e) {
      console.error("[ClaudeAccount] auth status", e);
      set({ authStatus: { loggedIn: false } });
    }
  },

  login: async () => {
    clearLoginPolling();
    set({ loginPending: true, loginUri: null, loginError: null });
    try {
      const info = await invoke<{ verificationUri: string | null }>(
        "claude_login",
      );
      set({ loginUri: info.verificationUri ?? null });
      _loginPollInterval = setInterval(async () => {
        try {
          const status = await invoke<ClaudeAuthStatus>("claude_auth_status");
          if (status.loggedIn && status.email) {
            clearLoginPolling();
            set({ authStatus: status, loginPending: false, loginUri: null });
            // Refresh dependent data
            void get().fetchProfile();
            void get().fetchModels();
          }
        } catch (e) {
          console.error("[ClaudeAccount] poll", e);
        }
      }, 2000);
      _loginPollTimeout = setTimeout(() => {
        clearLoginPolling();
        if (get().loginPending) set({ loginPending: false, loginUri: null });
      }, 180000);
    } catch (e) {
      const msg = String(e);
      console.error("[ClaudeAccount] login", msg);
      set({ loginPending: false, loginUri: null, loginError: msg });
    }
  },

  cancelLogin: async () => {
    clearLoginPolling();
    try {
      await invoke<boolean>("claude_cancel_login");
    } catch (e) {
      console.error("[ClaudeAccount] cancelLogin", e);
    }
    set({ loginPending: false, loginUri: null });
  },

  logout: async () => {
    try {
      await invoke("claude_logout");
      set({
        authStatus: { loggedIn: false },
        profile: null,
        usage: null,
        models: [],
      });
    } catch (e) {
      console.error("[ClaudeAccount] logout", e);
    }
  },
}));
