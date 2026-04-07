import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  buildProviderHistoryOptions,
  NEW_PROVIDER_SESSION_VALUE,
} from "@/components/AgentStatus/provider-session-view-model";
import type { ProviderHistoryInfo } from "@/stores/task-store/types";

function makeEntry(
  overrides: Partial<ProviderHistoryInfo> = {},
): ProviderHistoryInfo {
  return {
    provider: "claude",
    externalId: "sess_abc123456789",
    title: "Fix routing bug",
    normalizedTaskId: null,
    normalizedSessionId: null,
    archived: false,
    createdAt: 1,
    updatedAt: 2,
    status: "active",
    ...overrides,
  };
}

describe("buildProviderHistoryOptions", () => {
  test("New session option has no description", () => {
    const options = buildProviderHistoryOptions("claude", []);
    const newOpt = options.find((o) => o.value === NEW_PROVIDER_SESSION_VALUE);
    expect(newOpt?.description).toBeUndefined();
  });

  test("history option description contains truncated id and date", () => {
    const options = buildProviderHistoryOptions("claude", [
      makeEntry({ externalId: "sess_abc123456789" }),
    ]);
    const hist = options.find((o) => o.value !== NEW_PROVIDER_SESSION_VALUE);
    expect(hist?.description).toContain("sess_a…6789");
    expect(hist?.description).toContain("·");
  });
});

describe("CyberSelect rendering", () => {
  test("selected option with description shows two lines in collapsed state", async () => {
    const { CyberSelect } = await import("./cyber-select");
    const html = renderToStaticMarkup(
      createElement(CyberSelect, {
        value: "hist_1",
        options: [
          { value: "hist_1", label: "Fix routing bug", description: "task-42" },
          { value: NEW_PROVIDER_SESSION_VALUE, label: "New session" },
        ],
        onChange: () => {},
      }),
    );
    expect(html).toContain("Fix routing bug");
    expect(html).toContain("task-42");
  });

  test("selected option without description shows single label, no spurious content", async () => {
    const { CyberSelect } = await import("./cyber-select");
    const html = renderToStaticMarkup(
      createElement(CyberSelect, {
        value: "lead",
        options: [
          { value: "lead", label: "lead" },
          { value: "coder", label: "coder" },
        ],
        onChange: () => {},
      }),
    );
    expect(html).toContain("lead");
    expect(html).not.toContain("undefined");
    expect(html).not.toContain("null");
  });
});

describe("CyberSelect history variant menu items", () => {
  test("history menu option renders full text without truncation classes", async () => {
    const { HistoryMenuOption } = await import("./cyber-select");
    const longLabel =
      "A very long session title that would be clipped in a narrow container";
    const longDesc = "sess_abc123456789_very_long_external_id_overflow";
    const html = renderToStaticMarkup(
      createElement(HistoryMenuOption, {
        opt: { value: "h1", label: longLabel, description: longDesc },
        isSelected: false,
        onClick: () => {},
      }),
    );
    expect(html).toContain(longLabel);
    expect(html).toContain(longDesc);
  });

  test("history menu option avoids truncation classes and panel uses wide layout", async () => {
    const { HistoryMenuOption, getCyberSelectMenuPanelClassName } =
      await import("./cyber-select");
    const html = renderToStaticMarkup(
      createElement(HistoryMenuOption, {
        opt: {
          value: "h1",
          label: "A very long session title that would be clipped in a narrow container",
          description: "sess_abc123456789_very_long_external_id_overflow",
        },
        isSelected: false,
        onClick: () => {},
      }),
    );
    expect(html).not.toContain("truncate");
    expect(getCyberSelectMenuPanelClassName("history")).toContain("w-[22rem]");
  });
});

describe("CyberSelect history variant", () => {
  test("history variant keeps collapsed trigger compact", async () => {
    const { CyberSelect } = await import("./cyber-select");
    const html = renderToStaticMarkup(
      createElement(CyberSelect, {
        value: "hist_1",
        variant: "history",
        options: [
          {
            value: "hist_1",
            label: "A very long session title that should stay readable",
            description: "sess_abc123456789",
          },
        ],
        onChange: () => {},
      }),
    );
    expect(html).toContain("A very long session title");
    expect(html).not.toContain("sess_abc123456789");
  });
});
