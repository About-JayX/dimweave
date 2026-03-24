export interface BridgeMessage {
  id: string;
  from: string;
  to: string;
  content: string;
  timestamp: number;
  type?: "task" | "review" | "result" | "question" | "system";
  replyTo?: string;
  priority?: "normal" | "urgent";
}

// JSON-RPC 2.0

export interface JsonRpcRequest {
  jsonrpc?: "2.0";
  method: string;
  id: number;
  params?: Record<string, any>;
}

export interface JsonRpcResponse {
  jsonrpc?: "2.0";
  id: number;
  result?: any;
  error?: { code: number; message: string; data?: any };
}

export interface JsonRpcNotification {
  jsonrpc?: "2.0";
  method: string;
  params?: Record<string, any>;
}

export type JsonRpcMessage =
  | JsonRpcRequest
  | JsonRpcResponse
  | JsonRpcNotification;

// Codex App Server Types

export interface CodexThread {
  id: string;
}

export interface CodexItem {
  id: string;
  type: string;
  content?: Array<{ type: string; text?: string }>;
}

export interface CodexTurn {
  id: string;
}

// MCP Tool Schema

export interface McpTool {
  name: string;
  description: string;
  inputSchema: Record<string, any>;
}
