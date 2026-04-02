import { useEffect, useMemo } from "react";
import { Virtuoso } from "react-virtuoso";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectMessages } from "@/stores/bridge-store/selectors";
import { useTaskStore } from "@/stores/task-store";
import { selectActiveTask } from "@/stores/task-store/selectors";
import { MessageList } from "./MessageList";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import {
  filterRenderableChatMessages,
  formatTerminalTimestamp,
} from "./view-model";
import { TerminalSquare } from "lucide-react";
import type { ShellMainSurface } from "@/components/shell-layout-state";

interface MessagePanelProps {
  surfaceMode: ShellMainSurface;
}

export function MessagePanel({ surfaceMode }: MessagePanelProps) {
  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const messages = useBridgeStore(selectMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const claudeNeedsAttention = useBridgeStore((s) => s.claudeNeedsAttention);
  const clearClaudeAttention = useBridgeStore((s) => s.clearClaudeAttention);
  const activeTask = useTaskStore(selectActiveTask);
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
    if (claudeNeedsAttention) {
      clearClaudeAttention();
    }
  }, [claudeNeedsAttention, clearClaudeAttention]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center gap-3 border-b border-border/45 px-4 py-3">
        <div className="min-w-0 flex-1">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
            {surfaceMode === "logs" ? "Diagnostics" : "Conversation"}
          </div>
          <div className="mt-0.5 flex min-w-0 items-center gap-2">
            <div className="truncate text-sm font-semibold text-foreground">
              {surfaceMode === "logs" ? "Runtime logs" : "Primary timeline"}
            </div>
            <span className="rounded-full border border-border/45 px-2 py-0.5 text-[10px] text-muted-foreground">
              {surfaceMode === "logs"
                ? `${allTerminalLines.length} lines`
                : `${chatMessages.length} messages`}
            </span>
            {surfaceMode === "chat" && activeTask && (
              <span className="hidden truncate rounded-full border border-border/45 px-2 py-0.5 text-[10px] text-muted-foreground lg:inline-flex">
                {activeTask.title}
              </span>
            )}
            {surfaceMode === "chat" && reviewBadge && (
              <ReviewGateBadge badge={reviewBadge} />
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {surfaceMode === "logs" && errorLines.length > 0 && (
            <span className="inline-flex items-center gap-1 rounded-full border border-destructive/30 bg-destructive/8 px-2.5 py-1 text-[10px] font-medium text-destructive">
              <TerminalSquare className="size-3" />
              {errorLines.length} errors
            </span>
          )}
          {surfaceMode === "chat" && (
            <Button variant="secondary" size="xs" onClick={clearMessages}>
              Clear
            </Button>
          )}
        </div>
      </div>

      {surfaceMode === "chat" && <MessageList messages={chatMessages} />}

      {surfaceMode === "logs" && (
        <div className="flex-1 min-h-0">
          {allTerminalLines.length === 0 && (
            <div className="py-10 text-center font-sans text-[13px] text-muted-foreground">
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
                  <span className="mr-2 opacity-50">
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
    </div>
  );
}
