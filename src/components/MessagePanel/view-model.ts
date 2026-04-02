import type { BridgeMessage } from "@/types";
import type {
  ClaudeStreamState,
  CodexStreamState,
} from "@/stores/bridge-store/types";

export type StreamIndicatorId = "claude" | "codex";
export type MessagePanelTab = "messages" | "logs" | "approvals";
const DEFAULT_LOG_TIME_FORMATTER = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
});

export interface CodexStreamIndicatorViewModel {
  visible: boolean;
  hasVisibleContent: boolean;
  animatePulse: boolean;
  showStatusLabel: boolean;
  statusLabel: string;
}

export interface MessageListDisplayState {
  timelineCount: number;
  streamRailIndicators: StreamIndicatorId[];
  hasContent: boolean;
}

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
  const codexIndicator = getCodexStreamIndicatorViewModel(codexStream);
  return [
    ...(claudeStream.thinking ? (["claude"] as const) : []),
    ...(codexIndicator.visible
      ? (["codex"] as const)
      : []),
  ];
}

export function getMessageListDisplayState(
  messageCount: number,
  streamRailIndicators: StreamIndicatorId[],
): MessageListDisplayState {
  return {
    timelineCount: messageCount,
    streamRailIndicators,
    hasContent: messageCount > 0 || streamRailIndicators.length > 0,
  };
}

export function getStreamTextTail(text: string, maxChars: number): string {
  if (text.length <= maxChars) {
    return text;
  }

  return `…${text.slice(-maxChars)}`;
}

export function formatTerminalTimestamp(
  timestamp: number,
  formatter: Intl.DateTimeFormat = DEFAULT_LOG_TIME_FORMATTER,
): string {
  return formatter.format(timestamp);
}

export function getCodexStreamIndicatorViewModel(
  codexStream: CodexStreamState,
): CodexStreamIndicatorViewModel {
  const hasVisibleContent = Boolean(
    codexStream.currentDelta ||
      codexStream.activity ||
      codexStream.reasoning ||
      codexStream.commandOutput,
  );
  const statusLabel = codexStream.currentDelta
    ? "streaming…"
    : codexStream.activity
      ? codexStream.activity
      : "thinking…";

  return {
    visible: codexStream.thinking || hasVisibleContent,
    hasVisibleContent,
    animatePulse: !hasVisibleContent,
    showStatusLabel: codexStream.thinking || Boolean(codexStream.activity),
    statusLabel,
  };
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

  if (tab === "messages") {
    return {
      nextTab: null,
      clearStoreAttention: true,
    };
  }

  return {
    nextTab: "messages",
    clearStoreAttention: true,
  };
}
