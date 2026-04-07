import type { BridgeMessage } from "@/types";
import type {
  ClaudeStreamState,
  CodexStreamState,
} from "@/stores/bridge-store/types";
import type { ShellMainSurface } from "@/components/shell-layout-state";
import { hasMessagePayload } from "@/lib/message-payload";
export {
  filterMessagesByQuery,
  getExpandableTextState,
  getMessageSearchSummary,
  getStreamTextTail,
  type ExpandableTextState,
} from "./text-tools";

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

export function getSearchQueryForDisclosure(
  searchOpen: boolean,
  searchQuery: string,
): string {
  return searchOpen ? searchQuery : "";
}

export function shouldAutoScrollLogsOnSurfaceChange(
  previousSurface: ShellMainSurface | null,
  nextSurface: ShellMainSurface,
  lineCount: number,
): boolean {
  return previousSurface !== "logs" && nextSurface === "logs" && lineCount > 0;
}

export function getLogsFollowOutputMode(atBottom: boolean): false | "smooth" {
  return atBottom ? "smooth" : false;
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
      message.from !== "system" &&
      hasMessagePayload(message.content, message.attachments),
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

export interface MessageListDisplayStateInput {
  messageCount: number;
  hasClaudeDraft: boolean;
  streamRailIndicators: StreamIndicatorId[];
}

export function getMessageListDisplayState(
  input: MessageListDisplayStateInput,
): MessageListDisplayState {
  const { messageCount, hasClaudeDraft, streamRailIndicators } = input;
  return {
    timelineCount: messageCount + (hasClaudeDraft ? 1 : 0),
    streamRailIndicators,
    hasContent:
      messageCount > 0 || hasClaudeDraft || streamRailIndicators.length > 0,
  };
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
