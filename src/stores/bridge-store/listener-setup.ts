import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BridgeState } from "./types";
import {
  type AgentMessagePayload,
  type AgentStatusPayload,
  type ClaudeStreamPayload,
  type CodexStreamPayload,
  type PermissionPromptPayload,
  type SystemLogPayload,
} from "./listener-payloads";
import {
  handleClaudeStreamEvent,
  handleCodexStreamEvent,
  resetClaudeStream,
} from "./stream-reducers";

type BridgeSetter = (fn: (state: BridgeState) => Partial<BridgeState>) => void;

type NextLogId = () => number;

export function createBridgeListeners(
  set: BridgeSetter,
  nextLogId: NextLogId,
): Promise<UnlistenFn[]> {
  return Promise.all([
    listen<AgentMessagePayload>("agent_message", (e) => {
      set((s) => ({
        messages: [...s.messages.slice(-999), e.payload.payload],
      }));
    }),
    listen<SystemLogPayload>("system_log", (e) => {
      const { level, message } = e.payload;
      set((s) => ({
        terminalLines: [
          ...s.terminalLines.slice(-200),
          {
            id: nextLogId(),
            agent: "system",
            kind: level === "error" ? ("error" as const) : ("text" as const),
            line: message,
            timestamp: Date.now(),
          },
        ],
      }));
    }),
    listen<AgentStatusPayload>("agent_status", (e) => {
      const { agent, online, providerSession } = e.payload;
      set((s) => ({
        agents: {
          ...s.agents,
          [agent]: {
            ...s.agents[agent],
            name: agent,
            displayName: s.agents[agent]?.displayName ?? agent,
            status: online ? ("connected" as const) : ("disconnected" as const),
            providerSession: online ? providerSession : undefined,
          },
        },
        ...(agent === "claude" && !online
          ? { claudeStream: resetClaudeStream(s) }
          : {}),
      }));
    }),
    listen<ClaudeStreamPayload>("claude_stream", (e) => {
      set((s) => handleClaudeStreamEvent(s, e.payload));
    }),
    listen<CodexStreamPayload>("codex_stream", (e) => {
      set((s) => handleCodexStreamEvent(s, e.payload));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => ({
        permissionPrompts: [
          ...s.permissionPrompts.filter(
            (prompt) => prompt.requestId !== e.payload.requestId,
          ),
          e.payload,
        ],
      }));
    }),
  ]);
}
