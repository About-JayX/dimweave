import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ClaudeConfigRows } from "../src/components/ClaudePanel/ClaudeConfigRows";
import { CodexConfigRows } from "../src/components/AgentStatus/CodexConfigRows";

describe("agent workspace config rows", () => {
  test("claude configuration rows no longer render Select project", () => {
    const html = renderToStaticMarkup(
      <ClaudeConfigRows
        model=""
        effort=""
        cwd="/repo"
        disabled
        onModelChange={() => {}}
        onEffortChange={() => {}}
      />,
    );

    expect(html).not.toContain('title="/repo"><svg');
    expect(html).not.toContain("Select project...");
    expect(html).toContain("/repo");
  });

  test("codex configuration rows show a read-only workspace label", () => {
    const html = renderToStaticMarkup(
      <CodexConfigRows
        locked={false}
        profile={null}
        models={[]}
        selectedModel=""
        modelSelectOptions={[]}
        handleModelChange={() => {}}
        reasoningOptions={[]}
        selectedReasoning=""
        setSelectedReasoning={() => {}}
        reasoningSelectOptions={[]}
        cwd="/repo"
      />,
    );

    expect(html).not.toContain('title="/repo"><svg');
    expect(html).not.toContain("Select project...");
    expect(html).toContain("/repo");
  });
});
