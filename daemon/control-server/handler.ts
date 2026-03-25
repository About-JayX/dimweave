import type { ServerWebSocket } from "bun";
import type { ControlClientMessage } from "../control-protocol";
import { state, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";
import { attachAgent, detachAgent } from "./agent-session";
import {
  sendStatus,
  sendProtocolMessage,
  routeMessage,
} from "./message-routing";

export function handleControlMessage(
  ws: ServerWebSocket<ControlSocketData>,
  raw: string | Buffer,
  deps: ControlServerDeps,
) {
  let message: ControlClientMessage;
  try {
    message = JSON.parse(typeof raw === "string" ? raw : raw.toString());
  } catch {
    return;
  }

  switch (message.type) {
    case "agent_connect":
      attachAgent(ws, message.agentId, deps);
      return;
    case "agent_disconnect":
      detachAgent(ws, message.agentId, "requested disconnect", deps);
      return;
    case "status":
      sendStatus(ws, deps);
      return;
    case "fetch_messages": {
      const messages = state.flushBufferedMessages();
      sendProtocolMessage(ws, {
        type: "fetch_messages_result",
        requestId: message.requestId,
        messages,
      });
      return;
    }
    case "route_message": {
      const msg = message.message;

      // Resolve sender's agent id from role
      const senderAgentId =
        msg.from === state.claudeRole
          ? "claude"
          : msg.from === state.codexRole
            ? "codex"
            : null;

      if (!senderAgentId) {
        sendProtocolMessage(ws, {
          type: "route_result",
          requestId: message.requestId,
          success: false,
          error: `Invalid sender: ${msg.from} is not an assigned role`,
        });
        return;
      }

      deps.log(`Routing ${msg.from} → ${msg.to} (${msg.content.length} chars)`);

      const result = routeMessage(msg, { skipSender: senderAgentId });

      sendProtocolMessage(ws, {
        type: "route_result",
        requestId: message.requestId,
        success: result.success,
        error: result.error,
      });
      return;
    }
  }
}
