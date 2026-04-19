import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ClaudeModel {
  slug: string;
  displayName: string;
  supportedEfforts: string[];
}

interface ClaudeAccountState {
  models: ClaudeModel[];
  fetchFailed: boolean;
  fetchModels: () => Promise<void>;
}

export const useClaudeAccountStore = create<ClaudeAccountState>((set) => ({
  models: [],
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
}));
