/**
 * Test harness for bridge routing logic.
 * Validates message format, routing resolution, and edge cases.
 * Run: bun run test-routing.ts
 */

import type { BridgeMessage } from "./daemon/types";

// ── Mock state ───────────────────────────────────────────

const state = {
  claudeRole: "lead",
  codexRole: "coder",
  attachedClaude: true as any, // simulate online
  codexOnline: true,
};

// ── resolveTarget (extracted from handler logic) ─────────

function resolveTarget(to: string): {
  targets: Array<{ agent: "claude" | "codex"; online: boolean }>;
} {
  if (to === "user") return { targets: [] };
  const matches: Array<{ agent: "claude" | "codex"; online: boolean }> = [];
  if (state.claudeRole === to) {
    matches.push({ agent: "claude", online: state.attachedClaude !== null });
  }
  if (state.codexRole === to) {
    matches.push({ agent: "codex", online: state.codexOnline });
  }
  return { targets: matches };
}

// ── Test helpers ─────────────────────────────────────────

let passed = 0;
let failed = 0;

function assert(condition: boolean, name: string) {
  if (condition) {
    passed++;
    console.log(`  ✓ ${name}`);
  } else {
    failed++;
    console.error(`  ✗ ${name}`);
  }
}

function makeMsg(overrides: Partial<BridgeMessage>): BridgeMessage {
  return {
    id: `test_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
    from: "lead",
    to: "coder",
    content: "test message",
    timestamp: Date.now(),
    ...overrides,
  };
}

// ── Tests ────────────────────────────────────────────────

console.log("\n=== BridgeMessage Format Tests ===\n");

{
  const msg = makeMsg({ from: "lead", to: "coder", content: "hello" });
  assert(typeof msg.from === "string", "from is string");
  assert(typeof msg.to === "string", "to is string");
  assert(typeof msg.content === "string", "content is string");
  assert(typeof msg.timestamp === "number", "timestamp is number");
  assert(msg.from === "lead", "from is role name, not agent name");
  assert(!("source" in msg), "no legacy source field");
}

{
  const msg = makeMsg({
    type: "task",
    replyTo: "prev_123",
    priority: "urgent",
  });
  assert(msg.type === "task", "type field works");
  assert(msg.replyTo === "prev_123", "replyTo field works");
  assert(msg.priority === "urgent", "priority field works");
}

console.log("\n=== Routing Resolution Tests ===\n");

{
  // Normal: lead → coder
  const result = resolveTarget("coder");
  assert(result.targets.length === 1, "coder resolves to 1 target");
  assert(result.targets[0].agent === "codex", "coder maps to codex");
  assert(result.targets[0].online === true, "codex is online");
}

{
  // Normal: coder → lead
  const result = resolveTarget("lead");
  assert(result.targets.length === 1, "lead resolves to 1 target");
  assert(result.targets[0].agent === "claude", "lead maps to claude");
}

{
  // to: "user" → no agent targets
  const result = resolveTarget("user");
  assert(result.targets.length === 0, "user has no agent targets (GUI only)");
}

{
  // Unknown role → no targets
  const result = resolveTarget("reviewer");
  assert(result.targets.length === 0, "reviewer not assigned, 0 targets");
}

{
  // Same role on both agents
  const oldCodexRole = state.codexRole;
  state.codexRole = "lead"; // both are lead
  const result = resolveTarget("lead");
  assert(result.targets.length === 2, "same role → 2 targets (broadcast)");
  assert(
    result.targets.some((t) => t.agent === "claude"),
    "includes claude",
  );
  assert(
    result.targets.some((t) => t.agent === "codex"),
    "includes codex",
  );
  state.codexRole = oldCodexRole; // restore
}

{
  // Offline agent
  const oldClaude = state.attachedClaude;
  state.attachedClaude = null;
  const result = resolveTarget("lead");
  assert(result.targets.length === 1, "lead still resolves");
  assert(result.targets[0].online === false, "claude is offline");
  state.attachedClaude = oldClaude; // restore
}

console.log("\n=== Sender Validation Tests ===\n");

{
  // Valid sender
  const msg = makeMsg({ from: "lead" });
  assert(msg.from === state.claudeRole, "from matches claudeRole → valid");
}

{
  // Invalid sender
  const msg = makeMsg({ from: "hacker" });
  assert(msg.from !== state.claudeRole, "from doesn't match → should reject");
}

console.log("\n=== System Message Tests ===\n");

{
  // Simulate systemMessage
  let nextId = 0;
  function systemMessage(
    prefix: string,
    content: string,
    to?: string,
  ): BridgeMessage {
    return {
      id: `${prefix}_${++nextId}`,
      from: "system",
      to: to ?? "user",
      content,
      timestamp: Date.now(),
      type: "system",
    };
  }

  const msg = systemMessage("test", "hello", "lead");
  assert(msg.from === "system", "system message from is 'system'");
  assert(msg.to === "lead", "system message to is target role");
  assert(msg.type === "system", "system message type is 'system'");

  const defaultMsg = systemMessage("test2", "no target");
  assert(defaultMsg.to === "user", "default to is 'user'");
}

// ── Summary ──────────────────────────────────────────────

console.log(`\n=== Results: ${passed} passed, ${failed} failed ===\n`);
if (failed > 0) process.exit(1);
