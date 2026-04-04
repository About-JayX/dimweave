import { expect, test } from "@playwright/test";

const workspacePath = "/tmp/agent-bridge-e2e";
const workspaceName = "agent-bridge-e2e";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(
    ({ stubbedWorkspace, stubbedWorkspaceName }) => {
      let callbackId = 0;
      let activeTask: null | {
        taskId: string;
        workspaceRoot: string;
        title: string;
        status: string;
        reviewStatus: null;
        leadSessionId: null;
        currentCoderSessionId: null;
        createdAt: number;
        updatedAt: number;
      } = null;

      Object.assign(window, {
        __TAURI_INTERNALS__: {
          metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main" },
          },
          convertFileSrc: (filePath: string) => filePath,
          invoke: async (command: string, args?: Record<string, unknown>) => {
            if (command === "plugin:event|listen") return ++callbackId;
            if (command === "plugin:event|unlisten") return null;
            if (command === "daemon_get_status_snapshot") {
              return { agents: [], claudeRole: "lead", codexRole: "coder" };
            }
            if (command === "daemon_clear_active_task") {
              activeTask = null;
              return null;
            }
            if (command === "daemon_get_task_snapshot") {
              return activeTask
                ? { task: activeTask, sessions: [], artifacts: [] }
                : null;
            }
            if (command === "pick_directory") {
              return stubbedWorkspace;
            }
            if (command === "daemon_create_task") {
              activeTask = {
                taskId: "task_e2e",
                workspaceRoot: String(args?.workspace ?? stubbedWorkspace),
                title: stubbedWorkspaceName,
                status: "draft",
                reviewStatus: null,
                leadSessionId: null,
                currentCoderSessionId: null,
                createdAt: Date.now(),
                updatedAt: Date.now(),
              };
              return activeTask;
            }
            if (command === "daemon_list_provider_history") {
              return [];
            }
            return null;
          },
          transformCallback: () => ++callbackId,
          unregisterCallback: () => {},
        },
        __TAURI_EVENT_PLUGIN_INTERNALS__: {
          unregisterListener: () => {},
        },
      });
    },
    {
      stubbedWorkspace: workspacePath,
      stubbedWorkspaceName: workspaceName,
    },
  );
});

test("fresh boot can enter a workspace and land in the shell", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByRole("heading", { name: "Choose your workspace" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Choose folder..." }).click();

  await expect(page.locator("[data-workspace-selected='true']")).toContainText(
    workspacePath,
  );

  await page.getByRole("button", { name: "Continue" }).click();

  await expect(
    page.getByRole("heading", { name: "Choose your workspace" }),
  ).toHaveCount(0);
  await expect(page.getByTitle(workspacePath)).toContainText(workspaceName);
});
