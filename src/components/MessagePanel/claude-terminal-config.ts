import type { ITerminalOptions } from "@xterm/xterm";

export function createClaudeTerminalOptions(): ITerminalOptions {
  return {
    allowProposedApi: true,
    cursorBlink: true,
    fontFamily:
      '"Geist Mono Variable", "SFMono-Regular", Menlo, Monaco, Consolas, "Liberation Mono", monospace',
    fontSize: 13,
    fontWeight: "400",
    fontWeightBold: "600",
    lineHeight: 1.15,
    letterSpacing: 0,
    theme: {
      background: "#0b0d12",
      foreground: "#d8dee9",
      cursor: "#f97316",
      black: "#11131a",
      red: "#f87171",
      green: "#4ade80",
      yellow: "#facc15",
      blue: "#60a5fa",
      magenta: "#c084fc",
      cyan: "#22d3ee",
      white: "#e5e7eb",
      brightBlack: "#6b7280",
      brightWhite: "#f8fafc",
    },
    scrollback: 2000,
    allowTransparency: false,
    convertEol: false,
    customGlyphs: true,
    minimumContrastRatio: 1,
  };
}
