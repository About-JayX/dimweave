import type { BridgeMessage, CodexItem } from "../../types";
import type { CodexAccountInfo } from "../codex-adapter/codex-types";
import type { MessageHandlerCallbacks } from "./types";
import { captureTurnMetadata } from "./account-capture";

/**
 * Context required by notification handling — refs into the parent handler state.
 */
export interface NotificationHandlerState {
  agentMessageBuffers: Map<string, string[]>;
  activeTurnIds: Set<string>;
  turnInProgress: boolean;
  accountInfo: CodexAccountInfo;
}

/**
 * Process a Codex app-server notification (turn/started, item/*, turn/completed).
 */
export function handleNotification(
  msg: any,
  state: NotificationHandlerState,
  cb: MessageHandlerCallbacks,
): void {
  const { method, params } = msg;
  switch (method) {
    case "turn/started":
      markTurnStarted(state, params?.turn?.id);
      captureTurnMetadata(params?.turn, state.accountInfo, cb);
      cb.emitPhaseChanged("thinking");
      break;
    case "item/started": {
      const item: CodexItem = params?.item;
      if (item?.type === "reasoning") {
        cb.emitPhaseChanged("thinking");
      } else if (item?.type === "agentMessage") {
        cb.emitPhaseChanged("streaming");
        state.agentMessageBuffers.set(item.id, []);
        cb.emitAgentMessageStarted(item.id);
      }
      break;
    }
    case "item/agentMessage/delta": {
      const buf = state.agentMessageBuffers.get(params?.itemId);
      if (buf && params?.delta) {
        buf.push(params.delta);
        cb.emitAgentMessageDelta(params.itemId, params.delta);
      }
      break;
    }
    case "item/completed": {
      const item: CodexItem = params?.item;
      if (item?.type === "agentMessage") {
        const content = extractContent(item, state.agentMessageBuffers);
        state.agentMessageBuffers.delete(item.id);
        if (content) {
          cb.log(`Agent message completed (${content.length} chars)`);
          cb.emitAgentMessage({
            id: item.id,
            from: "codex",
            to: "",
            content,
            timestamp: Date.now(),
          } satisfies BridgeMessage);
        }
      }
      break;
    }
    case "turn/completed":
      markTurnCompleted(state, params?.turn?.id);
      captureTurnMetadata(params?.turn, state.accountInfo, cb);
      cb.emitPhaseChanged("idle");
      cb.emitTurnCompleted();
      break;
  }
}

// ── Helpers ──────────────────────────────────────────────

function extractContent(
  item: CodexItem,
  buffers: Map<string, string[]>,
): string {
  if (item.content?.length) {
    return item.content
      .filter((c) => c.type === "text" && c.text)
      .map((c) => c.text!)
      .join("");
  }
  return buffers.get(item.id)?.join("") ?? "";
}

function markTurnStarted(state: NotificationHandlerState, turnId?: string) {
  state.activeTurnIds.add(
    typeof turnId === "string" && turnId.length > 0
      ? turnId
      : `unknown:${Date.now()}`,
  );
  state.turnInProgress = state.activeTurnIds.size > 0;
}

function markTurnCompleted(state: NotificationHandlerState, turnId?: string) {
  if (typeof turnId === "string" && turnId.length > 0) {
    state.activeTurnIds.delete(turnId);
  } else {
    state.activeTurnIds.clear();
  }
  state.turnInProgress = state.activeTurnIds.size > 0;
}
