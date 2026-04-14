import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import { setupDOM, render, query, queryAll, click, teardownDOM } from "./dom-test-env";
import type { TaskAgentInfo } from "@/stores/task-store/types";

beforeEach(() => setupDOM());
afterEach(() => teardownDOM());

describe("TaskAgentEditor interaction", () => {
  test("edit-mode Save calls onSubmit preserving existing values", async () => {
    const onSubmit = mock(() => {});
    const agent: TaskAgentInfo = {
      agentId: "a1", taskId: "t1", provider: "codex",
      role: "coder", displayName: "My Coder", order: 0, createdAt: 1,
    };
    const { TaskAgentEditor } = await import("./TaskAgentEditor");
    await render(createElement(TaskAgentEditor, { agent, onSubmit, onCancel: () => {} }));

    const saveBtn = queryAll("button").find((b) => b.textContent === "Save");
    expect(saveBtn).toBeTruthy();
    click(saveBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.provider).toBe("codex");
    expect(payload.role).toBe("coder");
    expect(payload.displayName).toBe("My Coder");
  });

  test("cancel button calls onCancel", async () => {
    const onCancel = mock(() => {});
    const { TaskAgentEditor } = await import("./TaskAgentEditor");
    await render(createElement(TaskAgentEditor, { agent: null, onSubmit: () => {}, onCancel }));

    const cancelBtn = queryAll("button").find((b) => b.textContent === "Cancel");
    expect(cancelBtn).toBeTruthy();
    click(cancelBtn!);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  test("Add button disabled when role is empty", async () => {
    const onSubmit = mock(() => {});
    const { TaskAgentEditor } = await import("./TaskAgentEditor");
    await render(createElement(TaskAgentEditor, { agent: null, onSubmit, onCancel: () => {} }));

    const addBtn = queryAll("button").find((b) => b.textContent === "Add") as HTMLButtonElement;
    expect(addBtn).toBeTruthy();
    expect(addBtn.disabled).toBe(true);
    click(addBtn);
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
