import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { BackToBottomButton, MessageSearchChrome } from "./search-chrome";

describe("BackToBottomButton", () => {
  test("renders label and pill chrome", () => {
    const html = renderToStaticMarkup(
      createElement(BackToBottomButton, { onClick: () => {} }),
    );
    expect(html).toContain("Back to bottom");
    expect(html).toContain("rounded-full");
    expect(html).toContain("shadow-lg");
    expect(html).toContain("text-primary-foreground");
  });

  test("uses primary-tinted background (post-642ac2d polish)", () => {
    // The chrome was deliberately switched to `bg-primary/90` in commit
    // 642ac2d for better contrast against streaming content. If the design
    // shifts back to transparent, update this assertion — don't flip it
    // silently in search-chrome.tsx.
    const html = renderToStaticMarkup(
      createElement(BackToBottomButton, { onClick: () => {} }),
    );
    expect(html).toContain("bg-primary/90");
    expect(html).toContain("hover:bg-primary");
  });
});

describe("MessageSearchChrome", () => {
  test("closed state renders nothing — search row not disclosed", () => {
    const html = renderToStaticMarkup(
      createElement(MessageSearchChrome, {
        searchOpen: false,
        searchQuery: "",
        searchSummary: null,
        inputRef: { current: null },
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );
    expect(html).toBe("");
    expect(html).not.toContain('type="search"');
  });

  test("open state renders the search input", () => {
    const html = renderToStaticMarkup(
      createElement(MessageSearchChrome, {
        searchOpen: true,
        searchQuery: "",
        searchSummary: null,
        inputRef: { current: null },
        onOpen: () => {},
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );
    expect(html).toContain('type="search"');
  });
});
