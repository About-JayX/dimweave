import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { BackToBottomButton, MessageSearchChrome } from "./search-chrome";

describe("BackToBottomButton", () => {
  test("back-to-bottom button uses transparent chrome", () => {
    const html = renderToStaticMarkup(
      createElement(BackToBottomButton, { onClick: () => {} }),
    );
    expect(html).toContain("Back to bottom");
    expect(html).toContain("bg-transparent");
  });

  test("back-to-bottom button keeps the original chrome with a transparent background", () => {
    const html = renderToStaticMarkup(
      createElement(BackToBottomButton, { onClick: () => {} }),
    );
    expect(html).toContain("Back to bottom");
    expect(html).toContain("rounded-full");
    expect(html).toContain("text-primary-foreground");
    expect(html).toContain("shadow-lg");
    expect(html).toContain("bg-transparent");
    expect(html).not.toContain("bg-primary/90");
  });
});

describe("MessageSearchChrome", () => {
  test("closed state renders only the header search button", () => {
    const html = renderToStaticMarkup(
      createElement(MessageSearchChrome, {
        searchOpen: false,
        searchQuery: "",
        searchSummary: null,
        inputRef: { current: null },
        onOpen: () => {},
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );
    expect(html).toContain("Search messages");
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
