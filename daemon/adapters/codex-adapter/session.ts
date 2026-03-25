import type { AdapterState, InitSessionOptions } from "./types";
import { patchResponse } from "./codex-response-patcher";
import { ensureConnected } from "./lifecycle";
import type { EventEmitter } from "node:events";

export async function initSession(
  state: AdapterState,
  emitter: EventEmitter,
  log: (msg: string) => void,
  opts?: InitSessionOptions,
): Promise<{ success: boolean; error?: string }> {
  if (state.handler.activeThreadId) return { success: true };

  try {
    await ensureConnected(state, emitter, log);
  } catch (err: any) {
    return {
      success: false,
      error: `Cannot connect to app-server: ${err.message}`,
    };
  }

  if (!state.appServerWs || state.appServerWs.readyState !== WebSocket.OPEN) {
    return { success: false, error: "App-server WebSocket not connected" };
  }

  return new Promise((resolve) => {
    const timeout = setTimeout(
      () =>
        resolve({
          success: false,
          error: "Timeout waiting for thread creation",
        }),
      10000,
    );
    const initId = state.nextInjectionId++;
    const threadRpcId = state.nextInjectionId++;

    const handleMessage = (event: MessageEvent) => {
      const data =
        typeof event.data === "string" ? event.data : event.data.toString();
      try {
        const msg = JSON.parse(data);
        if (msg.id === initId) {
          if (msg.error) {
            if (!msg.error.message?.includes("Already initialized")) {
              log(`Initialize warning: ${msg.error.message}`);
            }
            // Capture patched init data (userAgent, platformOs, etc.)
            const patched = patchResponse(msg, data, log);
            if (patched !== data) {
              try {
                state.handler.intercept(JSON.parse(patched));
              } catch {}
            }
          } else {
            state.handler.intercept(msg);
          }
          state.appServerWs!.send(
            JSON.stringify({
              method: "thread/start",
              id: threadRpcId,
              params: {
                ...(opts?.model && { model: opts.model }),
                ...(opts?.reasoningEffort && {
                  reasoningEffort: opts.reasoningEffort,
                }),
                ...(opts?.cwd && { cwd: opts.cwd }),
                ...(opts?.sandboxMode && {
                  sandbox: opts.sandboxMode,
                }),
                ...(opts?.approvalPolicy && {
                  approvalPolicy: opts.approvalPolicy,
                }),
                ...(opts?.developerInstructions && {
                  settings: {
                    developer_instructions: opts.developerInstructions,
                  },
                }),
                dynamicTools: [
                  {
                    name: "reply",
                    description:
                      "Send a message to another agent role. Use get_status to see available roles.",
                    inputSchema: {
                      type: "object",
                      properties: {
                        to: {
                          type: "string",
                          description:
                            'Target role: "lead", "coder", "reviewer", "tester", or "user"',
                        },
                        text: {
                          type: "string",
                          description: "The message content",
                        },
                      },
                      required: ["to", "text"],
                    },
                  },
                  {
                    name: "check_messages",
                    description: "Check for new messages from other agents.",
                    inputSchema: { type: "object", properties: {} },
                  },
                  {
                    name: "get_status",
                    description:
                      "Get AgentBridge status: available roles and online agents.",
                    inputSchema: { type: "object", properties: {} },
                  },
                ],
              },
            }),
          );
        }
        if (msg.id === threadRpcId) {
          clearTimeout(timeout);
          state.appServerWs!.removeEventListener("message", handleMessage);
          // Capture model, modelProvider, serviceTier etc. from thread/start response
          state.handler.intercept(msg);
          const tid = msg.result?.thread?.id;
          if (tid) {
            state.handler.setActiveThreadId(tid, "initSession");
            // Trigger MCP server reload so app-server loads agentbridge from config.toml
            try {
              state.appServerWs!.send(
                JSON.stringify({
                  method: "config/mcpServer/reload",
                  id: state.nextInjectionId++,
                }),
              );
            } catch {}
            resolve({ success: true });
          } else {
            resolve({
              success: false,
              error: msg.error?.message ?? "Failed to create thread",
            });
          }
        }
      } catch {}
    };

    state.appServerWs!.addEventListener("message", handleMessage);
    state.appServerWs!.send(
      JSON.stringify({
        method: "initialize",
        id: initId,
        params: {
          clientInfo: { name: "agentbridge", version: "0.1.0" },
          protocolVersion: "0.1.0",
          capabilities: { experimentalApi: true },
        },
      }),
    );
  });
}

export function injectMessage(
  state: AdapterState,
  log: (msg: string) => void,
  text: string,
): boolean {
  if (!state.handler.activeThreadId) {
    log("Cannot inject: no active thread");
    return false;
  }
  if (!state.appServerWs || state.appServerWs.readyState !== WebSocket.OPEN) {
    log("Cannot inject: app-server WebSocket not connected");
    return false;
  }
  if (state.handler.turnInProgress) {
    log(`WARNING: injecting while a turn is already active`);
  }
  log(`Injecting message into Codex (${text.length} chars)`);
  try {
    state.appServerWs.send(
      JSON.stringify({
        method: "turn/start",
        id: state.nextInjectionId++,
        params: {
          threadId: state.handler.activeThreadId,
          input: [{ type: "text", text }],
        },
      }),
    );
    return true;
  } catch (err: any) {
    log(`Injection send failed: ${err.message}`);
    return false;
  }
}
