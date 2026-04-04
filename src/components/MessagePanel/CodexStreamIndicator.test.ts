import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { CodexStreamState } from "@/stores/bridge-store/types";
import {
  getExpandableTextState,
  getCodexStreamIndicatorViewModel,
  getTransientIndicators,
} from "./view-model";

function baseStream(): CodexStreamState {
  return {
    thinking: true,
    currentDelta: "",
    lastMessage: "",
    turnStatus: "",
    activity: "",
    reasoning: "",
    commandOutput: "",
  };
}

describe("getCodexStreamIndicatorViewModel", () => {
  test("activity-only state disables pulse and shows the activity label", () => {
    const viewModel = getCodexStreamIndicatorViewModel({
      ...baseStream(),
      thinking: false,
      activity: "Running: ls -la",
    });

    expect(viewModel.statusLabel).toBe("Running: ls -la");
    expect(viewModel.animatePulse).toBe(false);
    expect(viewModel.showStatusLabel).toBe(true);
  });

  test("reasoning content counts as visible content", () => {
    const viewModel = getCodexStreamIndicatorViewModel({
      ...baseStream(),
      reasoning: "Thinking through the file layout",
    });

    expect(viewModel.hasVisibleContent).toBe(true);
    expect(viewModel.animatePulse).toBe(false);
  });

  test("activity-only codex state still inserts a transient codex indicator", () => {
    const indicators = getTransientIndicators(
      {
        thinking: false,
        previewText: "",
        lastUpdatedAt: 0,
      },
      {
        ...baseStream(),
        thinking: false,
        activity: "Running: ls -la",
      },
    );

    expect(indicators).toEqual(["codex"]);
  });

  test("long reasoning exposes a collapsed preview with an expansion affordance", () => {
    const state = getExpandableTextState("a".repeat(320), 300, false);

    expect(state.canExpand).toBe(true);
    expect(state.toggleLabel).toBe("View full reasoning");
    expect(state.text.startsWith("…")).toBe(true);
    expect(state.text.length).toBeLessThan(320);
  });

  test("expanded reasoning returns the full text and collapse affordance", () => {
    const reasoning = "Need to compare the workspace graph before writing files.";
    const state = getExpandableTextState(reasoning, 20, true);

    expect(state.canExpand).toBe(true);
    expect(state.toggleLabel).toBe("Collapse reasoning");
    expect(state.text).toBe(reasoning);
  });

  test("renders the reasoning toggle label in the stream indicator view", async () => {
    const [{ CodexStreamIndicatorView }, { getStreamSurfacePresentation }] =
      await Promise.all([
        import("./CodexStreamIndicator"),
        import("./surface-styles"),
      ]);
    const html = renderToStaticMarkup(
      createElement(CodexStreamIndicatorView, {
        currentDelta: "",
        displayCommandOutput: "",
        displayReasoning: getExpandableTextState("a".repeat(320), 300, false),
        reasoningExpanded: false,
        surface: getStreamSurfacePresentation("codex"),
        viewModel: getCodexStreamIndicatorViewModel(baseStream()),
        onToggleReasoning: () => {},
      }),
    );

    expect(html).toContain("View full reasoning");
  });
});
