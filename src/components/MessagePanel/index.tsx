import { useState, useEffect, useRef, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import type { BridgeMessage } from "@/types";
import { PermissionQueue } from "./PermissionQueue";
import { TabBtn } from "./TabBtn";
import { MessageList } from "./MessageList";
import { ClaudeTerminalPane } from "./ClaudeTerminalPane";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import {
  filterRenderableChatMessages,
  getClaudeAttentionResolution,
} from "./view-model";

type Tab = "messages" | "claude" | "logs" | "approvals";

interface MessagePanelProps {
  messages: BridgeMessage[];
  onTabChange?: (tab: Tab) => void;
}

export function MessagePanel({ messages, onTabChange }: MessagePanelProps) {
  const [tab, setTabState] = useState<Tab>("messages");
  const prevClaudeRef = useRef({ connected: false, chunks: 0 });
  const [claudeTabAttention, setClaudeTabAttention] = useState(false);
  const setTab = (t: Tab) => {
    setTabState(t);
    onTabChange?.(t);
  };
  const logRef = useRef<HTMLDivElement>(null);

  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const claudeTerminalChunks = useBridgeStore((s) => s.claudeTerminalChunks);
  const claudeTerminalRunning = useBridgeStore((s) => s.claudeTerminalRunning);
  const claudeTerminalDetail = useBridgeStore((s) => s.claudeTerminalDetail);
  const permissionPrompts = useBridgeStore((s) => s.permissionPrompts);
  const respondToPermission = useBridgeStore((s) => s.respondToPermission);
  const claudeConnected =
    useBridgeStore((s) => s.agents.claude?.status) === "connected";
  const claudeNeedsAttention = useBridgeStore((s) => s.claudeNeedsAttention);
  const claudeFocusNonce = useBridgeStore((s) => s.claudeFocusNonce);
  const clearClaudeAttention = useBridgeStore((s) => s.clearClaudeAttention);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const claudeTerminalAvailable =
    claudeConnected || claudeTerminalRunning || claudeTerminalChunks.length > 0;
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
    if (!prevClaudeRef.current.connected && claudeConnected)
      setClaudeTabAttention(true);
    if (
      claudeTerminalChunks.length > prevClaudeRef.current.chunks &&
      tab !== "claude"
    )
      setClaudeTabAttention(true);
    prevClaudeRef.current = {
      connected: claudeConnected,
      chunks: claudeTerminalChunks.length,
    };
  }, [claudeConnected, claudeTerminalChunks.length, tab]);

  useEffect(() => {
    if (tab === "claude") setClaudeTabAttention(false);
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
        {claudeTerminalAvailable && (
          <TabBtn active={tab === "claude"} onClick={() => setTab("claude")}>
            Claude Terminal
            {claudeTabAttention && (
              <span className="ml-2 inline-flex size-2 rounded-full bg-claude animate-pulse" />
            )}
          </TabBtn>
        )}
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
        <div
          ref={logRef}
          className="flex-1 overflow-y-auto px-4 py-2 font-mono text-[11px] leading-relaxed"
        >
          {allTerminalLines.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground font-sans">
              No logs.
            </div>
          )}
          {allTerminalLines.map((l) => (
            <div
              key={l.id}
              className={`py-0.5 ${l.kind === "error" ? "text-destructive" : "text-muted-foreground"}`}
            >
              <span className="opacity-50 mr-2">
                {new Date(l.timestamp).toLocaleTimeString()}
              </span>
              <span className="mr-1 text-secondary-foreground">
                [{l.agent}]
              </span>
              {l.line}
            </div>
          ))}
        </div>
      )}
      {tab === "claude" && claudeTerminalAvailable && (
        <ClaudeTerminalPane
          chunks={claudeTerminalChunks}
          connected={claudeConnected}
          running={claudeTerminalRunning}
          detail={claudeTerminalDetail}
          focusNonce={claudeFocusNonce}
        />
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
