import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ErrorLogDialog } from "./ErrorLogDialog";
import type { UiError } from "@/stores/bridge-store/types";

const SAMPLE_ERRORS: UiError[] = [
  { id: 1, message: "Component crashed", timestamp: 1700000000000 },
  {
    id: 2,
    message: "Render failed",
    componentStack: "\n    at Broken\n    at App",
    timestamp: 1700000001000,
  },
];

describe("ErrorLogDialog", () => {
  test("does not render when closed", () => {
    const html = renderToStaticMarkup(
      <ErrorLogDialog open={false} errors={[]} onClose={() => {}} onClear={() => {}} />,
    );
    expect(html).not.toContain("Error Log");
  });

  test("shows empty state when no errors", () => {
    const html = renderToStaticMarkup(
      <ErrorLogDialog open errors={[]} onClose={() => {}} onClear={() => {}} />,
    );
    expect(html).toContain("Error Log");
    expect(html).toContain("No errors recorded");
    expect(html).not.toContain("Clear all");
  });

  test("error display is independent of task-agent model", () => {
    // ErrorLogDialog must render errors regardless of task/agent state
    const errors: UiError[] = [
      { id: 99, message: "Agent crash without task", timestamp: 1700000099000 },
    ];
    const html = renderToStaticMarkup(
      <ErrorLogDialog open errors={errors} onClose={() => {}} onClear={() => {}} />,
    );
    expect(html).toContain("Agent crash without task");
    expect(html).toContain("Clear all");
  });

  test("renders list of UI errors with timestamps", () => {
    const html = renderToStaticMarkup(
      <ErrorLogDialog open errors={SAMPLE_ERRORS} onClose={() => {}} onClear={() => {}} />,
    );
    expect(html).toContain("Component crashed");
    expect(html).toContain("Render failed");
    expect(html).toContain("at Broken");
    expect(html).toContain("Clear all");
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
  });
});
