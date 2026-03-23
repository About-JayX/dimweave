import type { ServerWebSocket } from "bun";
import type { CodexAdapter } from "./adapters/codex-adapter";
import type { TuiConnectionState } from "./tui-connection-state";
import {
  state,
  sendGuiEvent,
  broadcastToGui,
  type GuiSocketData,
} from "./daemon-state";

/**
 * Send text to Claude PTY via GUI frontend → Tauri invoke("pty_write").
 * PTY is managed by Rust (portable-pty), so daemon sends a WS event
 * to exactly ONE GUI client which writes to the Rust PTY.
 * Returns false if no GUI client is connected.
 */
export function sendToClaudePty(text: string) {
  const clients = state.guiClients;
  if (clients.size === 0) return false;

  const event = JSON.stringify({
    type: "pty_inject",
    payload: { data: text + "\r" },
    timestamp: Date.now(),
  });

  // Send to only the first connected client to avoid duplicate writes
  const firstClient = clients.values().next().value;
  if (firstClient) {
    try {
      firstClient.send(event);
    } catch {
      return false;
    }
  }
  return true;
}

import { ROLES, type RoleId } from "./role-config";
import { state as daemonState } from "./daemon-state";

/** Broadcast role change result (success or failure with revert) */
function broadcastRoleChange(
  agent: string,
  role: string,
  level: "info" | "error",
  message: string,
) {
  broadcastToGui({
    type: "role_sync",
    payload: { agent, role },
    timestamp: Date.now(),
  });
  broadcastToGui({
    type: "system_log",
    payload: { level, message },
    timestamp: Date.now(),
  });
}

interface GuiServerDeps {
  codex: CodexAdapter;
  tuiState: TuiConnectionState;
  currentStatus: () => any;
  broadcastStatus: () => void;
  log: (msg: string) => void;
}

export function startGuiServer(port: number, deps: GuiServerDeps) {
  const { codex, tuiState, currentStatus, broadcastStatus, log } = deps;

  state.guiServer = Bun.serve({
    port,
    hostname: "127.0.0.1",
    fetch(req, server) {
      const url = new URL(req.url);
      const corsHeaders = {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, OPTIONS",
        "Access-Control-Allow-Headers": "*",
      };

      if (req.method === "OPTIONS")
        return new Response(null, { headers: corsHeaders });
      if (url.pathname === "/healthz")
        return Response.json(
          { ok: true, pid: process.pid },
          { headers: corsHeaders },
        );
      if (url.pathname === "/status")
        return Response.json(currentStatus(), { headers: corsHeaders });
      if (server.upgrade(req, { data: { clientId: 0 } })) return undefined;
      return new Response("AgentBridge GUI Server", { headers: corsHeaders });
    },
    websocket: {
      open: (ws: ServerWebSocket<GuiSocketData>) => {
        ws.data.clientId = ++state.nextGuiClientId;
        state.guiClients.add(ws);
        log(`GUI client connected (#${ws.data.clientId})`);
        sendGuiEvent(ws, {
          type: "daemon_status",
          payload: currentStatus(),
          timestamp: Date.now(),
        });
      },
      close: (ws: ServerWebSocket<GuiSocketData>) => {
        state.guiClients.delete(ws);
        log(`GUI client disconnected (#${ws.data.clientId})`);
      },
      message: (ws: ServerWebSocket<GuiSocketData>, raw) => {
        handleGuiMessage(ws, raw, deps);
      },
    },
  });
}

function handleGuiMessage(
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
      if (!tuiState.canReply()) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: { level: "error", message: "Codex is not ready." },
          timestamp: Date.now(),
        });
        return;
      }
      const injected = codex.injectMessage(message.content);
      if (injected) {
        broadcastToGui({
          type: "agent_message",
          payload: {
            id: `gui_${Date.now()}`,
            source: "claude",
            content: message.content,
            timestamp: Date.now(),
          },
          timestamp: Date.now(),
        });
      }
      return;
    }
    case "get_status":
      sendGuiEvent(ws, {
        type: "daemon_status",
        payload: currentStatus(),
        timestamp: Date.now(),
      });
      return;
    case "launch_codex_tui": {
      if (!state.codexBootstrapped) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: {
            level: "error",
            message: "Codex app-server is not ready yet.",
          },
          timestamp: Date.now(),
        });
        return;
      }
      if (codex.activeThreadId) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: {
            level: "warn",
            message: "Codex session is already active.",
          },
          timestamp: Date.now(),
        });
        return;
      }
      log("Initializing Codex session from GUI...");
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Connecting to Codex..." },
        timestamp: Date.now(),
      });

      const roleConf = ROLES[daemonState.codexRole];
      codex
        .initSession({
          developerInstructions: roleConf.developerInstructions,
          sandboxMode: roleConf.sandboxMode,
          approvalPolicy: roleConf.approvalPolicy,
        })
        .then((result) => {
          if (result.success) {
            log("Codex session initialized successfully");
            tuiState.markBridgeReady();
            broadcastToGui({
              type: "agent_status",
              payload: {
                agent: "codex",
                status: "connected",
                threadId: codex.activeThreadId,
              },
              timestamp: Date.now(),
            });
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "info",
                message: `Codex connected! Thread: ${codex.activeThreadId}`,
              },
              timestamp: Date.now(),
            });
            broadcastStatus();
          } else {
            log(`Codex session init failed: ${result.error}`);
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "error",
                message: `Codex connection failed: ${result.error}`,
              },
              timestamp: Date.now(),
            });
          }
        })
        .catch((err: any) => {
          const error = err instanceof Error ? err.message : String(err);
          log(`Codex session init threw: ${error}`);
          broadcastToGui({
            type: "system_log",
            payload: {
              level: "error",
              message: `Codex connection failed: ${error}`,
            },
            timestamp: Date.now(),
          });
        })
        .finally(() => broadcastStatus());
      return;
    }
    case "apply_config": {
      // Merge incoming partial config with current session params to avoid losing values
      const currentInfo = codex.accountInfo;
      const model = message.model ?? currentInfo.model;
      const reasoningEffort =
        message.reasoningEffort ?? currentInfo.reasoningEffort;
      const cwd = message.cwd ?? currentInfo.cwd;
      log(
        `Applying config: model=${model ?? "-"}, reasoning=${reasoningEffort ?? "-"}, cwd=${cwd ?? "-"}`,
      );

      // Disconnect current session
      codex.disconnect();
      tuiState.handleCodexExit();
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Reconnecting with new config..." },
        timestamp: Date.now(),
      });

      // Reconnect with new settings (merged with current values)
      codex
        .ensureConnected()
        .then(() => {
          const rc = ROLES[daemonState.codexRole];
          return codex.initSession({
            model,
            reasoningEffort,
            cwd,
            developerInstructions: rc.developerInstructions,
            sandboxMode: rc.sandboxMode,
            approvalPolicy: rc.approvalPolicy,
          });
        })
        .then((result) => {
          if (result.success) {
            tuiState.markBridgeReady();
            broadcastToGui({
              type: "agent_status",
              payload: {
                agent: "codex",
                status: "connected",
                threadId: codex.activeThreadId,
              },
              timestamp: Date.now(),
            });
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "info",
                message: `Config applied! Model: ${model ?? "default"}`,
              },
              timestamp: Date.now(),
            });
            broadcastStatus();
          } else {
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "error",
                message: `Config apply failed: ${result.error}`,
              },
              timestamp: Date.now(),
            });
          }
        })
        .catch((err: any) => {
          const error = err instanceof Error ? err.message : String(err);
          log(`Config apply threw: ${error}`);
          broadcastToGui({
            type: "system_log",
            payload: {
              level: "error",
              message: `Config apply failed: ${error}`,
            },
            timestamp: Date.now(),
          });
        })
        .finally(() => broadcastStatus());
      return;
    }
    case "set_agent_role": {
      const { agent, role } = message as { agent: string; role: RoleId };
      const oldRole =
        agent === "codex" ? daemonState.codexRole : daemonState.claudeRole;
      if (agent === "claude") {
        daemonState.claudeRole = role;
        broadcastRoleChange(
          agent,
          role,
          "info",
          `Role changed: ${agent} → ${role}`,
        );
      } else if (agent === "codex") {
        daemonState.codexRole = role;
        if (codex.activeThreadId) {
          const currentInfo = codex.accountInfo;
          codex.disconnect();
          tuiState.handleCodexExit();
          broadcastStatus();
          codex
            .ensureConnected()
            .then(() => {
              const roleConfig = ROLES[role as keyof typeof ROLES];
              return codex.initSession({
                model: currentInfo.model,
                reasoningEffort: currentInfo.reasoningEffort,
                cwd: currentInfo.cwd,
                developerInstructions: roleConfig.developerInstructions,
                sandboxMode: roleConfig.sandboxMode,
                approvalPolicy: roleConfig.approvalPolicy,
              });
            })
            .then((result) => {
              if (result.success) {
                tuiState.markBridgeReady();
                broadcastStatus();
                broadcastRoleChange(
                  agent,
                  role,
                  "info",
                  `Role changed: ${agent} → ${role}`,
                );
              } else {
                daemonState.codexRole = oldRole;
                broadcastRoleChange(
                  agent,
                  oldRole,
                  "error",
                  `Role change failed: ${result.error}`,
                );
              }
            })
            .catch((err: any) => {
              const error = err instanceof Error ? err.message : String(err);
              log(`Role change reconnect failed: ${error}`);
              daemonState.codexRole = oldRole;
              broadcastRoleChange(
                agent,
                oldRole,
                "error",
                `Role change reconnect failed: ${error}`,
              );
            });
        } else {
          broadcastRoleChange(
            agent,
            role,
            "info",
            `Role changed: ${agent} → ${role}`,
          );
        }
      }
      log(`Role change requested: ${agent} → ${role}`);
      return;
    }
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
