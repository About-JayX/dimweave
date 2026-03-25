import { describe, expect, test } from "bun:test";
import { Terminal } from "@xterm/xterm";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { createClaudeTerminalOptions } from "../src/components/MessagePanel/claude-terminal-config";

describe("createClaudeTerminalOptions", () => {
  test("enables proposed API required by Unicode11Addon", () => {
    const terminal = new Terminal(createClaudeTerminalOptions());

    expect(() => terminal.loadAddon(new Unicode11Addon())).not.toThrow();
  });
});
