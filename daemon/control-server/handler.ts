import type { ServerWebSocket } from "bun";
import type { ControlClientMessage } from "../control-protocol";
import { state, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";
import { attachClaude, detachClaude } from "./claude-session";
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
    case "claude_connect":
      attachClaude(ws, deps);
      return;
    case "claude_disconnect":
      detachClaude(ws, "frontend requested disconnect", deps);
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

      // Sender validation: from must match claudeRole
      if (msg.from !== state.claudeRole) {
        sendProtocolMessage(ws, {
          type: "route_result",
          requestId: message.requestId,
          success: false,
          error: `Invalid sender: ${msg.from} does not match claude role ${state.claudeRole}`,
        });
        return;
      }

      deps.log(`Routing ${msg.from} → ${msg.to} (${msg.content.length} chars)`);

      // Use shared routing — skip sending back to claude (the sender)
      const result = routeMessage(msg, deps, { skipSender: "claude" });

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
