export type RoleId = "lead" | "coder" | "reviewer" | "tester";

export interface AgentRole {
  id: RoleId;
  label: string;
  /** developer_instructions injected into Codex thread/start */
  developerInstructions: string;
  /** Codex sandbox mode (OS-enforced) */
  sandboxMode: "read-only" | "workspace-write" | "danger-full-access";
  /** Codex approval policy */
  approvalPolicy: "on-request" | "never";
  /** Whether shell tool is enabled for Codex */
  shellTool: boolean;
  /** Prompt appended when forwarding this agent's output to Lead */
  forwardPrompt: string;
  /** Claude --agents JSON config for this role (hard enforcement) */
  claudeAgent: {
    description: string;
    prompt: string;
    tools?: string;
    disallowedTools?: string;
    permissionMode?: string;
    model?: string;
  };
}

export const ROLES: Record<RoleId, AgentRole> = {
  lead: {
    id: "lead",
    label: "Lead",
    developerInstructions: "",
    sandboxMode: "workspace-write",
    approvalPolicy: "never",
    shellTool: true,
    forwardPrompt: "",
    claudeAgent: {
      description:
        "Lead agent: reviews plans, makes final decisions, executes code changes.",
      prompt: `You are the Lead agent in AgentBridge, a multi-agent collaboration system.

## Your Role
- You have full control: read, write, edit files, run commands.
- You receive outputs from other agents (Coder, Reviewer, Tester) via the terminal.
- Review their work, then decide: apply as-is, modify, or reject.
- You are the final decision maker.

## When you receive agent output
- If the plan/code looks good → execute it directly.
- If it needs changes → make the changes yourself.
- If it's wrong → ignore it and proceed with your own approach.`,
      permissionMode: "bypassPermissions",
    },
  },
  coder: {
    id: "coder",
    label: "Coder",
    developerInstructions: `You are a code implementation agent within AgentBridge.
Your job: write code, implement features, fix bugs based on the task given.
Generate clear, well-structured code with proper error handling.
Your output will be reviewed by a Lead agent who decides whether to apply it.`,
    sandboxMode: "workspace-write",
    approvalPolicy: "never",
    shellTool: true,
    forwardPrompt:
      "[Coder completed task]\nReview the implementation below and decide whether to apply, modify, or reject it.",
    claudeAgent: {
      description: "Coder agent: writes code, implements features.",
      prompt: `You are a Coder agent in AgentBridge.

## Your Role
- Write code, implement features, fix bugs.
- You CAN read, write, and edit files.
- You CAN run shell commands.
- Your output will be forwarded to the Lead agent for review.`,
      permissionMode: "bypassPermissions",
    },
  },
  reviewer: {
    id: "reviewer",
    label: "Reviewer",
    developerInstructions: `You are a code review agent within AgentBridge.
Your job: analyze code quality, find bugs, suggest improvements.
You CANNOT modify files (read-only sandbox enforced at OS level).
Provide specific, actionable feedback with file paths and line numbers.
Your review will be sent to the Lead agent for final decision.`,
    sandboxMode: "read-only",
    approvalPolicy: "on-request",
    shellTool: false,
    forwardPrompt:
      "[Reviewer completed review]\nReview results below. Apply suggested fixes if appropriate.",
    claudeAgent: {
      description:
        "Reviewer agent: reviews code quality, finds bugs. Read-only.",
      prompt: `You are a Reviewer agent in AgentBridge.

## Your Role
- Analyze code quality, find bugs, suggest improvements.
- You can ONLY read files. You CANNOT write, edit, or run destructive commands.
- Provide specific feedback with file paths and line numbers.
- Your review will be forwarded to the Lead agent.`,
      tools: "Read,Grep,Glob,WebSearch,WebFetch",
      disallowedTools: "Write,Edit,Bash,NotebookEdit",
      permissionMode: "plan",
    },
  },
  tester: {
    id: "tester",
    label: "Tester",
    developerInstructions: `You are a testing agent within AgentBridge.
Your job: run tests, verify functionality, report bugs.
You CANNOT modify files (read-only sandbox enforced at OS level).
You CAN run test commands (shell enabled, read-only).
Report test results with pass/fail status and error details.`,
    sandboxMode: "read-only",
    approvalPolicy: "on-request",
    shellTool: true,
    forwardPrompt:
      "[Tester completed testing]\nTest results below. Fix any failures if needed.",
    claudeAgent: {
      description:
        "Tester agent: runs tests, reports bugs. Cannot modify files.",
      prompt: `You are a Tester agent in AgentBridge.

## Your Role
- Run tests, verify functionality, report bugs.
- You CAN run shell commands (for running tests).
- You CANNOT write or edit files.
- Report test results with pass/fail status and error details.`,
      tools: "Read,Grep,Glob,Bash,WebSearch,WebFetch",
      disallowedTools: "Write,Edit,NotebookEdit",
      permissionMode: "plan",
    },
  },
};

/** Generate --agents JSON for Claude CLI */
export function buildClaudeAgentsJson(roleId: RoleId): string {
  const role = ROLES[roleId];
  const agent: Record<string, any> = {
    description: role.claudeAgent.description,
    prompt: role.claudeAgent.prompt,
  };
  if (role.claudeAgent.tools) agent.tools = role.claudeAgent.tools;
  if (role.claudeAgent.disallowedTools)
    agent.disallowedTools = role.claudeAgent.disallowedTools;
  if (role.claudeAgent.permissionMode)
    agent.permissionMode = role.claudeAgent.permissionMode;
  if (role.claudeAgent.model) agent.model = role.claudeAgent.model;

  return JSON.stringify({ [roleId]: agent });
}

export const ROLE_OPTIONS: { value: RoleId; label: string }[] = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
];
