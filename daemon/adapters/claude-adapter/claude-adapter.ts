import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import { z } from "zod";
import type { BridgeMessage } from "../../types";

export type ReplySender = (
  msg: BridgeMessage,
) => Promise<{ success: boolean; error?: string }>;
export type MessageFetcher = () => Promise<BridgeMessage[]>;
export type StatusFetcher = () => Promise<{
  bridgeReady: boolean;
  codexTuiRunning: boolean;
  threadId: string | null;
  claudeRole?: string;
  codexRole?: string;
  claudeOnline?: boolean;
  codexOnline?: boolean;
}>;

const LOG_FILE = "/tmp/agentbridge.log";

export class ClaudeAdapter extends EventEmitter {
  private server: McpServer;
  private replySender: ReplySender | null = null;
  private messageFetcher: MessageFetcher | null = null;
  private statusFetcher: StatusFetcher | null = null;
  private claudeRole = "lead";

  constructor() {
    super();
    this.server = new McpServer({ name: "agentbridge", version: "0.1.0" });
    this.registerTools();
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    this.log("MCP server connected (tools: reply, check_messages, get_status)");
    this.emit("ready");
  }

  setReplySender(sender: ReplySender) {
    this.replySender = sender;
  }
  setMessageFetcher(fetcher: MessageFetcher) {
    this.messageFetcher = fetcher;
  }
  setStatusFetcher(fetcher: StatusFetcher) {
    this.statusFetcher = fetcher;
  }
  setClaudeRole(role: string) {
    this.claudeRole = role;
  }

  private registerTools() {
    this.server.registerTool(
      "reply",
      {
        description:
          "Send a message to a target agent role. Use get_status to see available roles and their online status.",
        inputSchema: {
          to: z
            .string()
            .describe(
              'Target role: "lead", "coder", "reviewer", "tester", or "user".',
            ),
          text: z.string().describe("The message content."),
          type: z
            .enum(["task", "review", "result", "question"])
            .optional()
            .describe("Message intent."),
          replyTo: z
            .string()
            .optional()
            .describe("ID of the message being replied to."),
          priority: z
            .enum(["normal", "urgent"])
            .optional()
            .describe("Message priority. Defaults to normal."),
        },
      },
      async ({ to, text, type: msgType, replyTo, priority }) => {
        if (!this.replySender) {
          return {
            content: [{ type: "text", text: "Error: bridge not connected." }],
            isError: true,
          };
        }

        const msg: BridgeMessage = {
          id: `claude_${Date.now()}`,
          from: this.claudeRole,
          to,
          content: text,
          timestamp: Date.now(),
          type: msgType,
          replyTo,
          priority: priority ?? "normal",
        };

        const result = await this.replySender(msg);
        if (!result.success) {
          this.log(`Reply failed: ${result.error}`);
          return {
            content: [{ type: "text", text: `Error: ${result.error}` }],
            isError: true,
          };
        }

        return {
          content: [{ type: "text", text: `Message routed to ${to}.` }],
        };
      },
    );

    this.server.registerTool(
      "check_messages",
      {
        description:
          "Check for new messages from other agents. Returns any messages received since the last check.",
      },
      async () => {
        if (!this.messageFetcher) {
          return {
            content: [
              { type: "text", text: "No new messages (bridge not connected)." },
            ],
          };
        }

        const messages = await this.messageFetcher();
        if (messages.length === 0) {
          return {
            content: [{ type: "text", text: "No new messages." }],
          };
        }

        const formatted = messages
          .map((m) => {
            const time = new Date(m.timestamp).toLocaleTimeString();
            return `[${time}] ${m.from}: ${m.content}`;
          })
          .join("\n\n---\n\n");

        this.log(`Returning ${messages.length} messages to Claude`);
        return {
          content: [
            {
              type: "text",
              text: `${messages.length} new message(s):\n\n${formatted}`,
            },
          ],
        };
      },
    );

    this.server.registerTool(
      "get_status",
      {
        description:
          "Get AgentBridge status: available roles, online agents, and connection state.",
      },
      async () => {
        if (!this.statusFetcher) {
          return { content: [{ type: "text", text: "Bridge not connected." }] };
        }

        const status = await this.statusFetcher();
        const lines = [
          `Bridge ready: ${status.bridgeReady ? "yes" : "no"}`,
          status.threadId ? `Thread: ${status.threadId}` : "No active thread",
          "",
          "Available roles:",
        ];
        if (status.claudeRole) {
          lines.push(
            `  ${status.claudeRole} (claude) - ${status.claudeOnline ? "online" : "offline"}`,
          );
        }
        if (status.codexRole) {
          lines.push(
            `  ${status.codexRole} (codex) - ${status.codexOnline ? "online" : "offline"}`,
          );
        }

        return { content: [{ type: "text", text: lines.join("\n") }] };
      },
    );
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [ClaudeAdapter] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
