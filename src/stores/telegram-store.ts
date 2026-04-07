import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";

export interface TelegramStateInfo {
  enabled: boolean;
  connected: boolean;
  notificationsEnabled: boolean;
  tokenLabel?: string | null;
  botUsername?: string | null;
  pairedChatLabel?: string | null;
  pendingPairCode?: string | null;
  pendingPairExpiresAt?: number | null;
  lastError?: string | null;
  lastDeliveryAt?: number | null;
  lastInboundAt?: number | null;
}

interface TelegramStore {
  state: TelegramStateInfo | null;
  loading: boolean;
  error: string | null;
  fetchState: () => Promise<void>;
  saveConfig: (
    botToken: string,
    enabled: boolean,
    notificationsEnabled: boolean,
  ) => Promise<void>;
  generatePairCode: () => Promise<void>;
  clearPairing: () => Promise<void>;
  cleanup: () => void;
}

let unlistenFn: UnlistenFn | null = null;

export const useTelegramStore = create<TelegramStore>((set) => ({
  state: null,
  loading: false,
  error: null,

  fetchState: async () => {
    set({ loading: true, error: null });
    try {
      const state = await invoke<TelegramStateInfo>("telegram_get_state");
      set({ state, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  saveConfig: async (botToken, enabled, notificationsEnabled) => {
    set({ loading: true, error: null });
    try {
      const state = await invoke<TelegramStateInfo>("telegram_save_config", {
        botToken,
        enabled,
        notificationsEnabled,
      });
      set({ state, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  generatePairCode: async () => {
    set({ error: null });
    try {
      const state = await invoke<TelegramStateInfo>(
        "telegram_generate_pair_code",
      );
      set({ state });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearPairing: async () => {
    set({ error: null });
    try {
      const state = await invoke<TelegramStateInfo>("telegram_clear_pairing");
      set({ state });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  cleanup: () => {
    if (unlistenFn) {
      unlistenFn();
      unlistenFn = null;
    }
  },
}));

// Auto-subscribe to telegram_state events
void listen<TelegramStateInfo>("telegram_state", (e) => {
  useTelegramStore.setState({ state: e.payload });
}).then((fn) => {
  unlistenFn = fn;
});
