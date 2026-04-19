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

interface ClaudeAccountState {
  models: ClaudeModel[];
  profile: ClaudeProfile | null;
  profileError: string | null;
  usage: ClaudeUsage | null;
  usageError: string | null;
  usageRefreshing: boolean;
  fetchFailed: boolean;
  fetchModels: () => Promise<void>;
  fetchProfile: () => Promise<void>;
  refreshUsage: () => Promise<void>;
}

export const useClaudeAccountStore = create<ClaudeAccountState>((set, get) => ({
  models: [],
  profile: null,
  profileError: null,
  usage: null,
  usageError: null,
  usageRefreshing: false,
  fetchFailed: false,
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
}));
