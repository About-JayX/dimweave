import { type UnlistenFn } from "@tauri-apps/api/event";
import { useTaskStore } from "@/stores/task-store";
import type { BridgeState } from "./types";
import {
  createBridgeListeners,
  hydrateMessagesForTask,
} from "./listener-setup";

export let _unlisteners: UnlistenFn[] = [];
export let _logId = 0;
export function nextLogId(): number {
  return ++_logId;
}
export function clearUnlisteners() {
  _unlisteners.forEach((fn) => fn());
  _unlisteners = [];
}
export function setUnlisteners(fns: UnlistenFn[]) {
  _unlisteners.forEach((fn) => fn());
  _unlisteners = fns;
}

export function initListeners(
  set: (fn: (s: BridgeState) => Partial<BridgeState>) => void,
) {
  createBridgeListeners(set, nextLogId).then((fns) => {
    setUnlisteners(fns);
    // Cover the case where task-store bootstrap already set activeTaskId
    // before our subscribe callback was attached — otherwise initial
    // hydration is missed and the chat timeline boots empty.
    const currentTaskId = useTaskStore.getState().activeTaskId;
    if (currentTaskId) {
      void hydrateMessagesForTask(currentTaskId, set);
    }
  });
}

export function logError(
  set: (fn: (s: BridgeState) => Partial<BridgeState>) => void,
) {
  return (e: unknown) =>
    set((s) => ({
      terminalLines: [
        ...s.terminalLines.slice(-200),
        {
          id: nextLogId(),
          agent: "system",
          kind: "error" as const,
          line: `[Error] ${String(e)}`,
          timestamp: Date.now(),
        },
      ],
    }));
}
