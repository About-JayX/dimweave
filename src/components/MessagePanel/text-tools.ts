import type { BridgeMessage } from "@/types";
import { sourceRole } from "@/types";

export interface ExpandableTextState {
  text: string;
  canExpand: boolean;
  toggleLabel: string | null;
}

export function getStreamTextTail(text: string, maxChars: number): string {
  if (text.length <= maxChars) {
    return text;
  }

  return `…${text.slice(-maxChars)}`;
}

export function getExpandableTextState(
  text: string,
  maxChars: number,
  expanded: boolean,
): ExpandableTextState {
  if (text.length <= maxChars) {
    return {
      text,
      canExpand: false,
      toggleLabel: null,
    };
  }

  return {
    text: expanded ? text : getStreamTextTail(text, maxChars),
    canExpand: true,
    toggleLabel: expanded ? "Collapse reasoning" : "View full reasoning",
  };
}

export function filterMessagesByQuery(
  messages: BridgeMessage[],
  query: string,
): BridgeMessage[] {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return messages;
  }

  return messages.filter((message) => {
    const haystacks = [
      message.content,
      sourceRole(message.source),
      message.source.displaySource,
      ...(message.attachments?.map((attachment) => attachment.fileName) ?? []),
    ];
    return haystacks.some((value) => value?.toLowerCase().includes(needle));
  });
}

export function getMessageSearchSummary(
  query: string,
  matchCount: number,
): string | null {
  const trimmed = query.trim();
  if (!trimmed) {
    return null;
  }
  if (matchCount === 1) {
    return `1 result for ${trimmed}.`;
  }
  if (matchCount === 0) {
    return `No messages match ${trimmed}.`;
  }
  return `${matchCount} results for ${trimmed}.`;
}
