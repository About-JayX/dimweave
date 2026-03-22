import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import type { BridgeMessage } from "../types";

export type ReplySender = (msg: BridgeMessage) => Promise<{ success: boolean; error?: string }>;

const LOG_FILE = "/tmp/agentbridge.log";

export class ClaudeAdapter extends EventEmitter {
  private server: Server;
  private notificationSeq = 0;
  private sessionId: string;
  private replySender: ReplySender | null = null;

  constructor() {
    super();
    this.sessionId = `codex_${Date.now()}`;

    this.server = new Server(
      { name: "agentbridge", version: "0.1.0" },
      {
        capabilities: {
          experimental: { "claude/channel": {} },
          tools: {},
        },
        instructions: [
          "Messages from the Codex agent arrive as <channel source=\"agentbridge\" chat_id=\"...\" user=\"Codex\" ...>.",
          "Codex is an AI coding agent (OpenAI) running in a separate session on the same machine.",
          "When Codex sends a message, review it and respond using the reply tool — pass chat_id back.",
          "Use the reply tool to send instructions, code review feedback, or follow-up tasks to Codex.",
        ].join("\n"),
      },
    );

    this.setupHandlers();
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    this.log("MCP server connected (channel capability registered)");
    this.emit("ready");
  }

  setReplySender(sender: ReplySender) {
    this.replySender = sender;
  }

  async pushNotification(message: BridgeMessage) {
    const msgId = `codex_msg_${++this.notificationSeq}`;
    const ts = new Date(message.timestamp).toISOString();

    try {
      await this.server.notification({
        method: "notifications/claude/channel",
        params: {
          content: message.content,
          meta: {
            chat_id: this.sessionId,
            message_id: msgId,
            user: "Codex",
            user_id: "codex",
            ts,
            source_type: "codex",
          },
        },
      });
      this.log(`Pushed notification: ${msgId}`);
    } catch (e: any) {
      this.log(`Failed to push notification: ${e.message}`);
    }
  }

  private setupHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: [
        {
          name: "reply",
          description:
            "Send a message back to Codex. Your reply will be injected into the Codex session as a new user turn.",
          inputSchema: {
            type: "object" as const,
            properties: {
              chat_id: {
                type: "string",
                description: "The conversation to reply in (from the inbound <channel> tag).",
              },
              text: {
                type: "string",
                description: "The message to send to Codex.",
              },
            },
            required: ["chat_id", "text"],
          },
        },
      ],
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      if (name === "reply") {
        const text = (args as any)?.text;
        if (!text) {
          return {
            content: [{ type: "text" as const, text: "Error: missing required parameter 'text'" }],
            isError: true,
          };
        }

        const bridgeMsg: BridgeMessage = {
          id: (args as any)?.chat_id ?? `reply_${Date.now()}`,
          source: "claude",
          content: text,
          timestamp: Date.now(),
        };

        if (!this.replySender) {
          this.log("No reply sender registered");
          return {
            content: [{ type: "text" as const, text: "Error: bridge not initialized, cannot send reply." }],
            isError: true,
          };
        }

        const result = await this.replySender(bridgeMsg);
        if (!result.success) {
          this.log(`Reply delivery failed: ${result.error}`);
          return {
            content: [{ type: "text" as const, text: `Error: ${result.error}` }],
            isError: true,
          };
        }

        return {
          content: [{ type: "text" as const, text: "Reply sent to Codex." }],
        };
      }

      return {
        content: [{ type: "text" as const, text: `Unknown tool: ${name}` }],
        isError: true,
      };
    });
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [ClaudeAdapter] ${msg}\n`;
    process.stderr.write(line);
    try { appendFileSync(LOG_FILE, line); } catch {}
  }
}
