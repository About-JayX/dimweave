import { useState, useEffect, useMemo } from "react";
import { Virtuoso } from "react-virtuoso";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import type { BridgeMessage } from "@/types";
import { PermissionQueue } from "./PermissionQueue";
import { TabBtn } from "./TabBtn";
import { MessageList } from "./MessageList";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import {
  filterRenderableChatMessages,
  getClaudeAttentionResolution,
  formatTerminalTimestamp,
} from "./view-model";

type Tab = "messages" | "logs" | "approvals";

interface MessagePanelProps {
  messages: BridgeMessage[];
  onTabChange?: (tab: Tab) => void;
}

export function MessagePanel({ messages, onTabChange }: MessagePanelProps) {
  const [tab, setTabState] = useState<Tab>("messages");
  const setTab = (t: Tab) => {
    setTabState(t);
    onTabChange?.(t);
  };

  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const permissionPrompts = useBridgeStore((s) => s.permissionPrompts);
  const respondToPermission = useBridgeStore((s) => s.respondToPermission);
  const claudeNeedsAttention = useBridgeStore((s) => s.claudeNeedsAttention);
  const clearClaudeAttention = useBridgeStore((s) => s.clearClaudeAttention);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const activeTask = activeTaskId ? tasks[activeTaskId] : null;
  const reviewBadge = getReviewBadge(activeTask?.reviewStatus);

  const chatMessages = useMemo(
    () => filterRenderableChatMessages(messages),
    [messages],
  );
  const errorLines = useMemo(
    () => allTerminalLines.filter((l) => l.kind === "error"),
    [allTerminalLines],
  );

  useEffect(() => {
    const attention = getClaudeAttentionResolution(tab, claudeNeedsAttention);
    if (attention.nextTab) {
      setTab(attention.nextTab);
    }
    if (attention.clearStoreAttention) {
      clearClaudeAttention();
    }
  }, [tab, claudeNeedsAttention, clearClaudeAttention]);

  return (
    <div className="flex flex-1 flex-col min-h-0">
      <div className="flex items-center px-4 py-2 border-b border-border/50 gap-3 relative">
        <TabBtn active={tab === "messages"} onClick={() => setTab("messages")}>
          Messages ({chatMessages.length})
        </TabBtn>
        <TabBtn active={tab === "logs"} onClick={() => setTab("logs")}>
          Logs {errorLines.length > 0 && `(${errorLines.length})`}
        </TabBtn>
        <TabBtn
          active={tab === "approvals"}
          onClick={() => setTab("approvals")}
        >
          Approvals
          {permissionPrompts.length > 0 && ` (${permissionPrompts.length})`}
        </TabBtn>
        <div className="flex-1" />
        {activeTask && (
          <div className="hidden min-w-0 items-center gap-2 md:flex">
            <span className="truncate rounded-full border border-border/50 px-2 py-0.5 text-[10px] text-muted-foreground">
              Task: {activeTask.title}
            </span>
            {reviewBadge && <ReviewGateBadge badge={reviewBadge} />}
          </div>
        )}
        {tab !== "approvals" && (
          <Button variant="secondary" size="xs" onClick={clearMessages}>
            Clear
          </Button>
        )}
        <div className="absolute bottom-0 left-0 right-0 h-px bg-linear-to-r from-transparent via-primary/15 to-transparent" />
      </div>
      {tab === "messages" && <MessageList messages={chatMessages} />}
      {tab === "logs" && (
        <div className="flex-1 min-h-0">
          {allTerminalLines.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground font-sans">
              No logs.
            </div>
          )}
          {allTerminalLines.length > 0 && (
            <Virtuoso
              data={allTerminalLines}
              className="h-full px-4 py-2 font-mono text-[11px] leading-relaxed"
              increaseViewportBy={160}
              itemContent={(_, line) => (
                <div
                  className={`py-0.5 ${line.kind === "error" ? "text-destructive" : "text-muted-foreground"}`}
                >
                  <span className="opacity-50 mr-2">
                    {formatTerminalTimestamp(line.timestamp)}
                  </span>
                  <span className="mr-1 text-secondary-foreground">
                    [{line.agent}]
                  </span>
                  {line.line}
                </div>
              )}
            />
          )}
        </div>
      )}
      {tab === "approvals" && (
        <PermissionQueue
          prompts={permissionPrompts}
          onResolve={respondToPermission}
        />
      )}
    </div>
  );
}
