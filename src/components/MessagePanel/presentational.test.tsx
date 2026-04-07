import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MessageSearchChrome } from "./search-chrome";

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
