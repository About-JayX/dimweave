import type { ServerWebSocket } from "bun";
import {
  sendGuiEvent,
  broadcastToGui,
  state as daemonState,
  type GuiSocketData,
} from "../daemon-state";
import type { GuiServerDeps } from "./types";
import { handleLaunchCodexTui, handleApplyConfig } from "./codex-actions";
import { handleSetRole } from "./role-actions";

export function handleGuiMessage(
  ws: ServerWebSocket<GuiSocketData>,
  raw: string | Buffer,
  deps: GuiServerDeps,
) {
  const { codex, tuiState, currentStatus, broadcastStatus, log } = deps;
  let message: any;
  try {
    message = JSON.parse(typeof raw === "string" ? raw : raw.toString());
  } catch {
    return;
  }

  switch (message.type) {
    case "send_to_codex": {
      // Always show user message in the panel
      broadcastToGui({
        type: "agent_message",
        payload: {
          id: `gui_${Date.now()}`,
          from: "user",
          to: daemonState.codexRole,
          content: message.content,
          timestamp: Date.now(),
          type: "task",
        },
        timestamp: Date.now(),
      });

      if (!tuiState.canReply()) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: { level: "error", message: "Codex is not ready." },
          timestamp: Date.now(),
        });
        return;
      }
      codex.injectMessage(message.content);
      return;
    }
    case "get_status":
      sendGuiEvent(ws, {
        type: "daemon_status",
        payload: currentStatus(),
        timestamp: Date.now(),
      });
      return;
    case "launch_codex_tui":
      handleLaunchCodexTui(ws, deps);
      return;
    case "apply_config":
      handleApplyConfig(message, deps);
      return;
    case "set_role":
      handleSetRole(message, deps);
      return;
    case "stop_codex_tui": {
      log("Disconnecting Codex from GUI...");
      codex.disconnect();
      tuiState.handleCodexExit();
      broadcastToGui({
        type: "agent_status",
        payload: { agent: "codex", status: "disconnected" },
        timestamp: Date.now(),
      });
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Codex disconnected." },
        timestamp: Date.now(),
      });
      broadcastStatus();
      return;
    }
  }
}
