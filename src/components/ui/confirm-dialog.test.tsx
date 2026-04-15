import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import { setupDOM, render, query, click, teardownDOM } from "../TaskPanel/dom-test-env";

beforeEach(() => setupDOM());
afterEach(() => teardownDOM());

describe("ConfirmDialog", () => {
  test("renders title, description, and buttons when open", async () => {
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "Delete Task", description: "Are you sure?",
        onConfirm: () => {}, onCancel: () => {},
      }),
    );
    const dialog = query('[data-confirm-dialog="true"]');
    expect(dialog).toBeTruthy();
    expect(dialog!.textContent).toContain("Delete Task");
    expect(dialog!.textContent).toContain("Are you sure?");
  });

  test("does not render when closed", async () => {
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: false, title: "Delete Task", description: "Are you sure?",
        onConfirm: () => {}, onCancel: () => {},
      }),
    );
    expect(query('[data-confirm-dialog="true"]')).toBeFalsy();
  });

  test("confirm button calls onConfirm", async () => {
    const onConfirm = mock(() => {});
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "Delete", description: "Sure?",
        onConfirm, onCancel: () => {},
      }),
    );
    const confirmBtn = query('[data-confirm-action="true"]');
    expect(confirmBtn).toBeTruthy();
    click(confirmBtn!);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  test("cancel button calls onCancel", async () => {
    const onCancel = mock(() => {});
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "Delete", description: "Sure?",
        onConfirm: () => {}, onCancel,
      }),
    );
    const cancelBtn = query('[data-confirm-cancel="true"]');
    expect(cancelBtn).toBeTruthy();
    click(cancelBtn!);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  test("uses custom labels when provided", async () => {
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "T", description: "D",
        confirmLabel: "Yes, delete", cancelLabel: "No, keep",
        onConfirm: () => {}, onCancel: () => {},
      }),
    );
    const confirmBtn = query('[data-confirm-action="true"]');
    const cancelBtn = query('[data-confirm-cancel="true"]');
    expect(confirmBtn!.textContent).toBe("Yes, delete");
    expect(cancelBtn!.textContent).toBe("No, keep");
  });

  test("defaults to Delete and Cancel labels", async () => {
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "T", description: "D",
        onConfirm: () => {}, onCancel: () => {},
      }),
    );
    const confirmBtn = query('[data-confirm-action="true"]');
    const cancelBtn = query('[data-confirm-cancel="true"]');
    expect(confirmBtn!.textContent).toBe("Delete");
    expect(cancelBtn!.textContent).toBe("Cancel");
  });

  test("backdrop click calls onCancel", async () => {
    const onCancel = mock(() => {});
    const { ConfirmDialog } = await import("./confirm-dialog");
    await render(
      createElement(ConfirmDialog, {
        open: true, title: "T", description: "D",
        onConfirm: () => {}, onCancel,
      }),
    );
    // The backdrop is the first child div with bg-black class
    const backdrop = document.querySelector(".bg-black\\/40") as HTMLElement;
    expect(backdrop).toBeTruthy();
    click(backdrop);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});
