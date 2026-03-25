import type { BridgeMessage, AgentInfo } from "@/types";

export interface TerminalLine {
  agent: string;
  kind: string;
  line: string;
  timestamp: number;
}

export interface BridgeState {
  connected: boolean;
  messages: BridgeMessage[];
  agents: Record<string, AgentInfo>;
  terminalLines: TerminalLine[];
  claudeRole: string;
  codexRole: string;
  draft: string;

  setDraft: (text: string) => void;
  sendToCodex: (content: string) => void;
  clearMessages: () => void;
  launchCodexTui: () => void;
  stopCodexTui: () => void;
  applyConfig: (config: {
    model?: string;
    reasoningEffort?: string;
    cwd?: string;
  }) => void;
  setRole: (agent: "claude" | "codex", role: string) => void;
  cleanup: () => void;
}
