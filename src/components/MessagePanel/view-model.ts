import type { BridgeMessage } from "@/types";
import type {
  ClaudeStreamState,
  CodexStreamState,
} from "@/stores/bridge-store/types";

export type StreamIndicatorId = "claude" | "codex";
export type MessagePanelTab = "messages" | "claude" | "logs" | "approvals";

export function getMessageIdentityPresentation(
  message: BridgeMessage,
): {
  badgeSource: string;
  roleLabel: string | null;
} {
  const badgeSource = message.displaySource ?? message.from;
  const roleLabel =
    message.from !== badgeSource &&
    !["user", "system"].includes(message.from)
      ? message.from
      : null;
  return { badgeSource, roleLabel };
}

export function filterRenderableChatMessages(
  messages: BridgeMessage[],
): BridgeMessage[] {
  return messages.filter(
    (message) =>
      message.from !== "system" && message.content.trim().length > 0,
  );
}

export function getTransientIndicators(
  claudeStream: ClaudeStreamState,
  codexStream: CodexStreamState,
): StreamIndicatorId[] {
  return [
    ...(claudeStream.thinking ? (["claude"] as const) : []),
    ...(codexStream.thinking || !!codexStream.currentDelta
      ? (["codex"] as const)
      : []),
  ];
}

export function getClaudeAttentionResolution(
  tab: MessagePanelTab,
  needsAttention: boolean,
): {
  nextTab: MessagePanelTab | null;
  clearStoreAttention: boolean;
} {
  if (!needsAttention) {
    return {
      nextTab: null,
      clearStoreAttention: false,
    };
  }

  if (tab === "claude") {
    return {
      nextTab: null,
      clearStoreAttention: true,
    };
  }

  return {
    nextTab: "claude",
    clearStoreAttention: true,
  };
}

export function getClaudeTerminalPlaceholder(
  connected: boolean,
  running: boolean,
  chunkCount: number,
): string | null {
  if (chunkCount > 0) {
    return null;
  }
  if (running) {
    return "Claude terminal is starting. Waiting for output…";
  }
  if (connected) {
    return "Claude is connected. Waiting for terminal output…";
  }
  return "Claude terminal is idle. Connect Claude to start an embedded session.";
}
