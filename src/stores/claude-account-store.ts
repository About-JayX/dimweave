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

interface ClaudeAccountState {
  models: ClaudeModel[];
  profile: ClaudeProfile | null;
  profileError: string | null;
  fetchFailed: boolean;
  fetchModels: () => Promise<void>;
  fetchProfile: () => Promise<void>;
}

export const useClaudeAccountStore = create<ClaudeAccountState>((set) => ({
  models: [],
  profile: null,
  profileError: null,
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
}));
