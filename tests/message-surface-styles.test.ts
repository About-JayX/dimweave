import { describe, expect, test } from "bun:test";
import {
  getMessageSurfacePresentation,
  getStreamSurfacePresentation,
} from "../src/components/MessagePanel/surface-styles";
import { getSourceBadgePresentation } from "../src/components/MessagePanel/SourceBadge";

describe("message surface presentation", () => {
  test("returns per-source brand bg without shadows", () => {
    const user = getMessageSurfacePresentation("user");
    const claude = getMessageSurfacePresentation("claude");
    const codex = getMessageSurfacePresentation("codex");

    expect(user.containerClass).toContain("bg-sky-500/10");
    expect(claude.containerClass).toContain("bg-claude/8");
    expect(codex.containerClass).toContain("bg-codex/8");
    expect(user.containerClass).not.toContain("shadow");
  });

  test("falls back to system for unknown source", () => {
    const unknown = getMessageSurfacePresentation("unknown");
    expect(unknown.containerClass).toContain("bg-muted/40");
  });
});

describe("source badge presentation", () => {
  test("returns accent color class per source", () => {
    const claude = getSourceBadgePresentation("claude");
    const codex = getSourceBadgePresentation("codex");
    const user = getSourceBadgePresentation("user");

    expect(claude.className).toContain("text-claude");
    expect(codex.className).toContain("text-codex");
    expect(user.className).toContain("text-sky-400");
    expect(claude.className).not.toContain("shadow");
  });
});
