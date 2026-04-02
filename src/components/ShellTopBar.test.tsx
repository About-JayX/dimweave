import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ShellTopBar } from "./ShellTopBar";

describe("ShellTopBar", () => {
  test("renders a minimal product title and the current workspace", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar workspaceLabel="~/Desktop/figma" />,
    );

    expect(html).toContain("Dimweave");
    expect(html).toContain("Dimweave logo");
    expect(html).toContain("Current workspace");
    expect(html).toContain("~/Desktop/figma");
    expect(html).not.toContain("Logs");
    expect(html).not.toContain("Approvals");
  });
});
