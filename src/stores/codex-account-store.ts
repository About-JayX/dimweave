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

  fetchProfile: () => Promise<void>;
  fetchUsage: () => Promise<void>;
  refreshUsage: () => Promise<void>;
  fetchModels: () => Promise<void>;
  pickDirectory: () => Promise<string | null>;
  login: () => Promise<void>;
  cancelLogin: () => Promise<void>;
  logout: () => Promise<void>;
}

export type { CodexProfile, UsageSnapshot, CodexModel, ReasoningLevel };

export const useCodexAccountStore = create<CodexAccountState>((set, get) => ({
  profile: null,
  usage: null,
  models: [],
  loading: false,
  refreshing: false,
  loginPending: false,
  loginUri: null,

  fetchProfile: async () => {
    try {
      const profile = await invoke<CodexProfile>("get_codex_account");
      set({ profile });
    } catch {}
  },

  fetchUsage: async () => {
    if (get().loading) return;
    set({ loading: true });
    try {
      const usage = await invoke<UsageSnapshot>("refresh_usage");
      set({ usage, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  refreshUsage: async () => {
    set({ refreshing: true });
    try {
      const usage = await invoke<UsageSnapshot>("refresh_usage");
      set({ usage, refreshing: false });
    } catch {
      set({ refreshing: false });
    }
  },

  fetchModels: async () => {
    try {
      const models = await invoke<CodexModel[]>("list_codex_models");
      set({ models });
    } catch {}
  },

  pickDirectory: async () => {
    try {
      return await invoke<string | null>("pick_directory");
    } catch {
      return null;
    }
  },

  login: async () => {
    set({ loginPending: true, loginUri: null });
    try {
      const info = await invoke<OAuthLaunchInfo>("codex_login");
      set({ loginUri: info.verificationUri });
    } catch {
      set({ loginPending: false, loginUri: null });
    }
  },

  cancelLogin: async () => {
    try {
      await invoke<boolean>("codex_cancel_login");
    } catch {}
    set({ loginPending: false, loginUri: null });
  },

  logout: async () => {
    try {
      await invoke("codex_logout");
      set({ profile: null, usage: null });
    } catch {}
  },
}));
