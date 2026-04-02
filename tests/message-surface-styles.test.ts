import { describe, expect, test } from "bun:test";
import {
  getMessageSurfacePresentation,
  getStreamSurfacePresentation,
} from "../src/components/MessagePanel/surface-styles";
import { getSourceBadgePresentation } from "../src/components/MessagePanel/SourceBadge";

describe("message surface presentation", () => {
  test("softens user and agent bubbles without glow-heavy shells", () => {
    const user = getMessageSurfacePresentation(true);
    const agent = getMessageSurfacePresentation(false);

    expect(user.containerClass).not.toContain("shadow");
    expect(agent.containerClass).not.toContain("shadow");
    expect(user.containerClass).toContain("bg-sky-500/8");
    expect(agent.containerClass).toContain("bg-card/45");
  });

  test("uses dashed low-contrast draft shells for stream indicators", () => {
    const claude = getStreamSurfacePresentation("claude");
    const codex = getStreamSurfacePresentation("codex");

    expect(claude.containerClass).toContain("border-dashed");
    expect(codex.containerClass).toContain("border-dashed");
    expect(claude.containerClass).not.toContain("shadow");
    expect(codex.containerClass).not.toContain("shadow");
  });
});

describe("source badge presentation", () => {
  test("keeps provider badges lightweight and without glow shadows", () => {
    const claude = getSourceBadgePresentation("claude");
    const codex = getSourceBadgePresentation("codex");
    const user = getSourceBadgePresentation("user");

    expect(claude.className).not.toContain("shadow");
    expect(codex.className).not.toContain("shadow");
    expect(user.className).not.toContain("shadow");
    expect(claude.className).toContain("bg-claude/6");
    expect(codex.className).toContain("bg-codex/6");
    expect(user.className).toContain("bg-sky-500/6");
  });
});
