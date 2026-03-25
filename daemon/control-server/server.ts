import type { ServerWebSocket } from "bun";
import { state, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";
import { detachAgent } from "./agent-session";
import { handleControlMessage } from "./handler";

export function startControlServer(port: number, deps: ControlServerDeps) {
  const { log } = deps;

  state.controlServer = Bun.serve({
    port,
    hostname: "127.0.0.1",
    fetch(req, server) {
      const url = new URL(req.url);
      if (url.pathname === "/healthz" || url.pathname === "/readyz") {
        return Response.json(deps.currentStatus());
      }
      if (
        url.pathname === "/ws" &&
        server.upgrade(req, { data: { clientId: 0, attached: false } })
      ) {
        return undefined;
      }
      return new Response("AgentBridge daemon");
    },
    websocket: {
      open: (ws: ServerWebSocket<ControlSocketData>) => {
        ws.data.clientId = ++state.nextControlClientId;
        log(`Control socket opened (#${ws.data.clientId})`);
      },
      close: (ws: ServerWebSocket<ControlSocketData>) => {
        log(`Control socket closed (#${ws.data.clientId})`);
        // Detach any agent using this socket
        for (const [agentId, agentWs] of state.attachedAgents) {
          if (agentWs === ws) {
            detachAgent(ws, agentId, "socket closed", deps);
            break;
          }
        }
      },
      message: (ws: ServerWebSocket<ControlSocketData>, raw) => {
        handleControlMessage(ws, raw, deps);
      },
    },
  });
}
