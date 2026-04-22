# 2026-04-22 Claude 流式指示器回退 + 历史 session 无法 Connect

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 Claude 流式工作过程在 UI 上消失的回退、Claude 历史 session 点 Connect 不生效，并补齐排查中发现的 Codex 同类 stale-task launch 问题。

**Architecture:** 前端 `ClaudeStreamIndicator` / `CodexStreamIndicator` 改为订阅 **per-task 选择器** `makeActiveClaudeStreamSelector(activeTaskId)` / `makeActiveCodexStreamSelector(activeTaskId)`，不再把 `claudeStream` / `codexStream` singleton mirror 当作指示器的真实数据源。后端 `DaemonCmd::ResumeSession` 的 Claude 分支改用 `sess.task_id.clone()`（与 Codex 对齐），避免把 resume launch 绑到当前活动任务或空串任务。前端 `ClaudePanel.doLaunch` 与 `CodexPanel.handleConnect` 的 `useCallback` 依赖数组补齐 `activeTask?.taskId`，消除切 task 后首次 launch 命中旧任务的陈旧闭包。

**Tech Stack:** React 19 + Zustand (bridge-store / task-store), Rust Tokio daemon, Tauri 2, `bun:test`, `cargo test`

---

## 背景 / 根因

### 问题 1：Claude 流式"回退"

- `4acf53e` 首次把 `claude_stream` / `codex_stream` 事件包上 `taskId + agentId`，为 task-scoped stream 铺路。
- `8336d2b` 把 `claudeStreamsByTask` / `codexStreamsByTask` 变成真正的 per-task bucket，并让 singleton mirror 只做 active-task 镜像；这一步是这次 UI “指示器消失/失声”最可能的回归放大点。
- 后续 `f14857e`（补齐 remaining `None,None` emit sites）和 `814b36f`（bridge handshake carries task identity）继续修正 task stamping，但 `src/components/MessagePanel/ClaudeStreamIndicator.tsx` 和 `src/components/MessagePanel/CodexStreamIndicator.tsx` 仍只订阅 `s.claudeStream` / `s.codexStream` 这个 **singleton mirror**。
- 当前 mirror 仅在 `applyClaudeStreamToBucket` / `applyCodexStreamToBucket` 里看到 `activeId === taskId` 时才更新（`src/stores/bridge-store/listener-setup.ts:41-79`）。只要事件 `taskId` 和前端 `activeTaskId` 不完全一致，per-task bucket 明明有数据，indicator 也会完全看不到。
- 触发场景包括：bootstrap 期 active task 尚未水合、Claude 历史 resume 先用错误 task launch、用户切换 task 的瞬间 stream 事件落到另一个 bucket。
- `resolveStreamBucketId()` 现在已经把 falsy `taskId` 回退到当前 active task，因此空串 fallback 不是 stream 通道眼下的主缺口；真正缺口是 indicator 仍在读 singleton mirror。

### 问题 2：Claude 选历史 session 点 Connect 不生效

- 前端 `src/components/ClaudePanel/index.tsx:134-160` 的 `doLaunch` 逻辑：
  - `selectedHistory.normalizedSessionId` 存在 → 走 `resumeNormalized` → `daemon_resume_session(sessionId)`
  - 否则 → `resumeExternal` → `daemon_launch_claude_sdk({ resumeSessionId: externalId, taskId: activeTask?.taskId })`
- 后端 `src-tauri/src/daemon/mod.rs:1300-1346` 的 Claude `ResumeSession` 分支当前使用：
  ```rust
  let resume_task_id = state.read().await.active_task_id
      .clone().unwrap_or_default();
  ```
  同一函数里的 Codex 分支（约 line 1230）早已使用 `sess.task_id.clone()`。
- 副作用：
  - 若 `active_task_id` 为 `None`，`resume_task_id = ""`，Claude launch 会先落到不存在的 task slot；`claudeOnlineForTask` 永远不会对真实 task 翻成 `true`，按钮一直显示 `Connect`。
  - 若当前 active task 不是该历史 session 原生所属 task，resume 会先绑定到错误 task，再由 `resume_session()` 把 state 指针切回原 task；runtime slot 与 UI 绑定会出现错位。
- 额外：`doLaunch` 的 `useCallback` deps 漏了 `activeTask?.taskId`，切 task 后第一次点击可能复用上一次 taskId。

### 问题 3：Codex 对照排查结果

- `src-tauri/src/daemon/mod.rs` 的 Codex `ResumeSession` 分支已经使用 `sess.task_id.clone()`，因此 **没有** Claude 那种 backend history-resume 绑错 task 的问题。
- 但 `src/components/AgentStatus/CodexPanel.tsx` 的 `handleConnect` 和 ClaudePanel 一样，仍会在 `resumeExternal` / 新建 launch 路径里捕获陈旧的 `activeTask?.taskId`。所以本次一并补齐它的 `useCallback` 依赖数组。

### 非目标

- ❌ 不重构 singleton mirror 机制本身（保留以兼容现有 `listener-setup.test.ts` fixture 与 `baseState()`）
- ❌ 不动 `ResumeSession` 里 Codex 后端分支（已正确）
- ❌ 不动 `agent_message` / `permission_prompt` 通道（已在 `0aa03a4` / `ba3a6d3` 修过）
- ❌ 不引入 `daemon_resume_session` 新命令或 DTO 变更
- ❌ 不为了这次修复去大改 daemon command loop 的可测试性结构

### 测试策略说明

- 当前 Claude history-resume 的真 bug 位于 `DaemonCmd::ResumeSession` command handler 内部，而现有 `DaemonState::resume_session()` 已经有正确的 state-layer 测试。
- 因此 **不新增** 那种“state 层继续绿、但 handler 仍然错”的伪回归测试。Rust 侧依赖现有 daemon suite 保证不回归，真实行为由手工 E2E 和 task-scoped UI 观察验证。

## 文件结构

| 文件 | 角色 |
|---|---|
| `src-tauri/src/daemon/mod.rs` | `DaemonCmd::ResumeSession` 的 Claude 分支改用 `sess.task_id.clone()` |
| `src/stores/bridge-store/selectors.ts` | 新增 `makeActiveClaudeStreamSelector` / `makeActiveCodexStreamSelector` 工厂 |
| `src/stores/bridge-store/selectors.test.ts`（新建） | 选择器按 taskId 取 bucket、缺桶返回稳定 default 的单元测试 |
| `src/components/MessagePanel/ClaudeStreamIndicator.tsx` | 用 `makeActiveClaudeStreamSelector(activeTaskId)` 取状态，不再读 singleton |
| `src/components/MessagePanel/CodexStreamIndicator.tsx` | 用 `makeActiveCodexStreamSelector(activeTaskId)` 取状态，不再读 singleton |
| `src/components/ClaudePanel/index.tsx` | `doLaunch` useCallback deps 补 `activeTask?.taskId` |
| `src/components/AgentStatus/CodexPanel.tsx` | `handleConnect` useCallback deps 补 `activeTask?.taskId` |
| `docs/agents/claude-chain.md` | 追加 Claude fix 记录，并注明 Codex backend 对照结果 |
| `docs/superpowers/plans/2026-04-22-claude-stream-fix-and-history-resume.md` | 本 plan 的 CM 回填 |

---

## Task 1 — 修复 Claude `ResumeSession` 绑定错误 task

**Files:**
- Modify: `src-tauri/src/daemon/mod.rs`（Claude resume 分支，约 line 1300–1346）

- [ ] **Step 1: 重新定位 Claude resume 分支**

运行：

```bash
cd /Users/jason/floder/agent-bridge && rg -n "Provider::Claude" src-tauri/src/daemon/mod.rs
```

预期：能定位到 `DaemonCmd::ResumeSession` 里 Claude 分支，看到当前仍在读取 `active_task_id.unwrap_or_default()`。

- [ ] **Step 2: 把 `active_task_id` 替换为 `sess.task_id`**

编辑 `src-tauri/src/daemon/mod.rs`，将：

```rust
let resume_task_id = state.read().await.active_task_id
    .clone().unwrap_or_default();
```

替换成：

```rust
// Align with Codex resume: pin the launch to the session's origin task,
// not whatever task happens to be active in the UI when the user clicks
// Connect on a history entry.
let resume_task_id = sess.task_id.clone();
```

- [ ] **Step 3: 重跑 Rust 聚焦验证**

运行：

```bash
cd /Users/jason/floder/agent-bridge && cargo test -p dimweave state_task_snapshot_tests
cd /Users/jason/floder/agent-bridge && cargo test -p dimweave claude_tests
```

预期：两组测试都通过；不新增 Rust 失败。

- [ ] **Step 4: 类型检查前端无影响**

运行：

```bash
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
```

预期：没有新错误（该改动只在 Rust 层）。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/daemon/mod.rs
git commit -m "$(cat <<'EOF'
fix(daemon): Claude resume binds to sess.task_id, matching Codex

Previously the Claude branch of DaemonCmd::ResumeSession read
active_task_id.unwrap_or_default(), which produced empty-string task
ids when no task was active and rebound history resumes to the wrong
task when the user was viewing a different one.

Align with the Codex branch which already uses sess.task_id.clone().

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 — 新增 per-task stream 选择器（失败测试 → 实现）

**Files:**
- Modify: `src/stores/bridge-store/selectors.ts`
- Create: `src/stores/bridge-store/selectors.test.ts`

- [ ] **Step 1: 写失败测试**

创建 `src/stores/bridge-store/selectors.test.ts`：

```ts
import { describe, expect, test } from "bun:test";
import {
  makeActiveClaudeStreamSelector,
  makeActiveCodexStreamSelector,
} from "./selectors";
import type { BridgeState } from "./types";

function baseState(): BridgeState {
  return {
    connected: true,
    messagesByTask: {},
    agents: {},
    terminalLines: [],
    uiErrors: [],
    permissionPrompts: [],
    permissionError: null,
    runtimeHealth: null,
    claudeNeedsAttention: false,
    claudeRole: "",
    codexRole: "",
    claudeStream: {
      thinking: false,
      previewText: "",
      thinkingText: "",
      blockType: "idle",
      toolName: "",
      lastUpdatedAt: 0,
    },
    codexStream: {
      thinking: false,
      currentDelta: "",
      lastMessage: "",
      turnStatus: "",
      activity: "",
      reasoning: "",
      commandOutput: "",
    },
    claudeStreamsByTask: {},
    codexStreamsByTask: {},
    draft: "",
    setDraft: () => {},
    clearClaudeAttention: () => {},
    sendToCodex: () => {},
    clearMessages: () => {},
    stopCodexTui: () => {},
    respondToPermission: async () => {},
    applyConfig: async () => {},
    pushUiError: () => {},
    clearUiErrors: () => {},
    setRole: () => {},
    cleanup: () => {},
  };
}

describe("makeActiveClaudeStreamSelector", () => {
  test("returns the per-task bucket for the given taskId", () => {
    const state = baseState();
    state.claudeStreamsByTask = {
      t1: {
        thinking: true,
        previewText: "hello",
        thinkingText: "",
        blockType: "text",
        toolName: "",
        lastUpdatedAt: 100,
      },
    };
    const sel = makeActiveClaudeStreamSelector("t1");
    expect(sel(state).previewText).toBe("hello");
    expect(sel(state).thinking).toBe(true);
  });

  test("returns a stable default when bucket is missing", () => {
    const state = baseState();
    const sel = makeActiveClaudeStreamSelector("missing");
    const a = sel(state);
    const b = sel(state);
    expect(a.previewText).toBe("");
    expect(a.thinking).toBe(false);
    expect(a).toBe(b);
  });

  test("returns singleton mirror when taskId is null (bootstrap race)", () => {
    const state = baseState();
    state.claudeStream = {
      thinking: true,
      previewText: "from singleton",
      thinkingText: "",
      blockType: "text",
      toolName: "",
      lastUpdatedAt: 1,
    };
    const sel = makeActiveClaudeStreamSelector(null);
    expect(sel(state).previewText).toBe("from singleton");
  });
});

describe("makeActiveCodexStreamSelector", () => {
  test("returns the per-task bucket for the given taskId", () => {
    const state = baseState();
    state.codexStreamsByTask = {
      t1: {
        thinking: true,
        currentDelta: "draft",
        lastMessage: "",
        turnStatus: "",
        activity: "",
        reasoning: "",
        commandOutput: "",
      },
    };
    const sel = makeActiveCodexStreamSelector("t1");
    expect(sel(state).currentDelta).toBe("draft");
  });

  test("returns stable default when bucket is missing", () => {
    const state = baseState();
    const sel = makeActiveCodexStreamSelector("missing");
    const a = sel(state);
    const b = sel(state);
    expect(a.currentDelta).toBe("");
    expect(a).toBe(b);
  });
});
```

- [ ] **Step 2: 运行并确认失败**

运行：

```bash
cd /Users/jason/floder/agent-bridge && bun test src/stores/bridge-store/selectors.test.ts
```

预期：`makeActiveClaudeStreamSelector is not a function` / `makeActiveCodexStreamSelector is not a function`。

- [ ] **Step 3: 实现选择器**

编辑 `src/stores/bridge-store/selectors.ts`：

1. 在文件顶部 import 区加入：

```ts
import {
  defaultClaudeStreamState,
  defaultCodexStreamState,
} from "./stream-reducers";
```

2. 在文件末尾追加：

```ts
const DEFAULT_CLAUDE_STREAM = defaultClaudeStreamState();
const DEFAULT_CODEX_STREAM = defaultCodexStreamState();

export function makeActiveClaudeStreamSelector(
  taskId: string | null,
): (state: BridgeState) => BridgeState["claudeStream"] {
  if (!taskId) return (state) => state.claudeStream;
  return (state) => state.claudeStreamsByTask[taskId] ?? DEFAULT_CLAUDE_STREAM;
}

export function makeActiveCodexStreamSelector(
  taskId: string | null,
): (state: BridgeState) => BridgeState["codexStream"] {
  if (!taskId) return (state) => state.codexStream;
  return (state) => state.codexStreamsByTask[taskId] ?? DEFAULT_CODEX_STREAM;
}
```

- [ ] **Step 4: 重跑并确认全绿**

运行：

```bash
cd /Users/jason/floder/agent-bridge && bun test src/stores/bridge-store/selectors.test.ts
```

预期：5 tests passed, 0 failed。

- [ ] **Step 5: TypeScript 全量检查**

运行：

```bash
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
```

预期：无新增错误。

- [ ] **Step 6: 提交**

```bash
git add src/stores/bridge-store/selectors.ts src/stores/bridge-store/selectors.test.ts
git commit -m "$(cat <<'EOF'
feat(bridge-store): add per-task stream selectors with stable defaults

Factory selectors read the active task's stream bucket directly instead
of relying on the singleton mirror, which only updates when activeId
matches event.taskId.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3 — `ClaudeStreamIndicator` 切换到 per-task 选择器

**Files:**
- Modify: `src/components/MessagePanel/ClaudeStreamIndicator.tsx`

- [ ] **Step 1: 用工厂选择器读取 active task bucket**

编辑 `src/components/MessagePanel/ClaudeStreamIndicator.tsx`，把顶部 import 和 state 读取改为：

```tsx
import { useEffect, useMemo, useState } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import { makeActiveClaudeStreamSelector } from "@/stores/bridge-store/selectors";
import { getExpandableTextState, getStreamTextTail } from "./view-model";
import { SourceBadge } from "./SourceBadge";
import { getStreamSurfacePresentation } from "./surface-styles";
import type { ClaudeBlockType } from "@/stores/bridge-store/types";

function blockLabel(blockType: ClaudeBlockType, toolName: string): string {
  switch (blockType) {
    case "thinking":
      return "thinking…";
    case "text":
      return "writing…";
    case "tool":
      return toolName ? `using ${toolName}` : "using tool…";
    default:
      return "thinking…";
  }
}

export function ClaudeStreamIndicator() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectClaudeStream = useMemo(
    () => makeActiveClaudeStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const stream = useBridgeStore(selectClaudeStream);
  const { thinking, previewText, thinkingText, blockType, toolName } = stream;
  const surface = getStreamSurfacePresentation("claude");
  const [thinkingExpanded, setThinkingExpanded] = useState(false);

  const displayText = useMemo(
    () => getStreamTextTail(previewText, 3000),
    [previewText],
  );
  const displayThinking = useMemo(
    () => getExpandableTextState(thinkingText, 300, thinkingExpanded),
    [thinkingText, thinkingExpanded],
  );

  useEffect(() => {
    setThinkingExpanded(false);
  }, [thinkingText]);

  if (!thinking && !previewText && !thinkingText) return null;

  const hasText = previewText.length > 0;
  const label = blockLabel(blockType, toolName);
  const isAnimating = blockType === "thinking" && !thinkingText;

  return (
    <div className="py-1.5">
      <div className="flex justify-start">
        <div className="max-w-[82%] rounded-xl bg-claude/8 px-3.5 py-2.5">
          <div className="flex items-center gap-2 mb-1">
            <SourceBadge source="claude" />
            <span
              className={`${surface.statusClass} ${isAnimating ? "animate-pulse" : ""}`}
            >
              {label}
            </span>
          </div>
          {hasText && <div className={surface.commandClass}>{displayText}</div>}
          {displayThinking.text && (
            <div
              className={`text-[11px] text-muted-foreground/50 italic whitespace-pre-wrap mt-1 ${
                thinkingExpanded ? "" : "max-h-24 overflow-hidden"
              }`}
            >
              {displayThinking.text}
            </div>
          )}
          {displayThinking.canExpand && (
            <button
              type="button"
              onClick={() => setThinkingExpanded((v) => !v)}
              className="mt-1 text-[11px] font-medium text-claude hover:text-claude/80 transition-colors active:scale-95"
            >
              {displayThinking.toggleLabel}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 类型检查**

```bash
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
```

预期：无新增错误。

- [ ] **Step 3: 运行 MessagePanel / bridge-store 相关测试**

```bash
cd /Users/jason/floder/agent-bridge && bun test src/components/MessagePanel/ src/stores/bridge-store/
```

预期：既有测试全部通过。

- [ ] **Step 4: 提交**

```bash
git add src/components/MessagePanel/ClaudeStreamIndicator.tsx
git commit -m "$(cat <<'EOF'
fix(stream): ClaudeStreamIndicator reads per-task bucket directly

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4 — `CodexStreamIndicator` 切换到 per-task 选择器

**Files:**
- Modify: `src/components/MessagePanel/CodexStreamIndicator.tsx`

- [ ] **Step 1: 用工厂选择器读取 active task bucket**

编辑 `src/components/MessagePanel/CodexStreamIndicator.tsx`，把顶部改成：

```tsx
import { useEffect, useMemo, useState } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import { makeActiveCodexStreamSelector } from "@/stores/bridge-store/selectors";
import { SourceBadge } from "./SourceBadge";
import {
  getExpandableTextState,
  getCodexStreamIndicatorViewModel,
  getStreamTextTail,
} from "./view-model";
import { getStreamSurfacePresentation } from "./surface-styles";

export function CodexStreamIndicator() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectCodexStream = useMemo(
    () => makeActiveCodexStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const stream = useBridgeStore(selectCodexStream);
  const { currentDelta, activity, reasoning, commandOutput } = stream;
  const codexStream = {
    thinking: stream.thinking,
    currentDelta,
    lastMessage: "",
    turnStatus: "",
    activity,
    reasoning,
    commandOutput,
  };
  const viewModel = getCodexStreamIndicatorViewModel(codexStream);
  const [reasoningExpanded, setReasoningExpanded] = useState(false);
  const displayReasoning = useMemo(
    () => getExpandableTextState(reasoning, 300, reasoningExpanded),
    [reasoning, reasoningExpanded],
  );
  const displayCommandOutput = useMemo(
    () => getStreamTextTail(commandOutput, 500),
    [commandOutput],
  );
  const surface = getStreamSurfacePresentation("codex");

  useEffect(() => {
    setReasoningExpanded(false);
  }, [reasoning]);

  if (!viewModel.visible) return null;

  return (
    <CodexStreamIndicatorView
      currentDelta={currentDelta}
      displayCommandOutput={displayCommandOutput}
      displayReasoning={displayReasoning}
      reasoningExpanded={reasoningExpanded}
      surface={surface}
      viewModel={viewModel}
      onToggleReasoning={() => setReasoningExpanded((value) => !value)}
    />
  );
}
```

保留 `CodexStreamIndicatorView` 组件（下半部分 JSX）原样。

- [ ] **Step 2: 类型检查**

```bash
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
```

预期：无新增错误。

- [ ] **Step 3: 运行测试**

```bash
cd /Users/jason/floder/agent-bridge && bun test src/components/MessagePanel/ src/stores/bridge-store/
```

预期：全绿。

- [ ] **Step 4: 提交**

```bash
git add src/components/MessagePanel/CodexStreamIndicator.tsx
git commit -m "$(cat <<'EOF'
fix(stream): CodexStreamIndicator reads per-task bucket directly

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5 — 修复 Claude/Codex 面板的陈旧 taskId 闭包

**Files:**
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`

- [ ] **Step 1: 补 `ClaudePanel.doLaunch` 的 deps**

把 `src/components/ClaudePanel/index.tsx` 里 `doLaunch` 结尾的 deps 数组：

```ts
}, [claudeRole, effectiveCwd, model, effort, selectedHistory, resumeSession]);
```

改为：

```ts
}, [
  claudeRole,
  effectiveCwd,
  model,
  effort,
  selectedHistory,
  resumeSession,
  activeTask?.taskId,
]);
```

- [ ] **Step 2: 补 `CodexPanel.handleConnect` 的 deps**

把 `src/components/AgentStatus/CodexPanel.tsx` 里 `handleConnect` 的 deps 数组：

```ts
  }, [
    applyConfig,
    resumeSession,
    selectedModel,
    selectedReasoning,
    effectiveCwd,
    selectedHistory,
  ]);
```

改为：

```ts
  }, [
    applyConfig,
    resumeSession,
    selectedModel,
    selectedReasoning,
    effectiveCwd,
    selectedHistory,
    activeTask?.taskId,
  ]);
```

- [ ] **Step 3: 类型检查**

```bash
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
```

预期：无新增错误。

- [ ] **Step 4: 运行相关前端测试**

```bash
cd /Users/jason/floder/agent-bridge && bun test src/components/AgentStatus/ src/components/ClaudePanel/ src/components/TaskPanel/
```

预期：没有新失败；若已有 baseline 失败，失败数不增加。

- [ ] **Step 5: 提交**

```bash
git add src/components/ClaudePanel/index.tsx src/components/AgentStatus/CodexPanel.tsx
git commit -m "$(cat <<'EOF'
fix(panels): include activeTask.taskId in Claude and Codex launch callbacks

Both provider panels were capturing stale activeTask?.taskId across task
switches. The first connect after switching tasks could launch against
the previously-active task on resumeExternal/new-session paths.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6 — 更新 `docs/agents/claude-chain.md` 修复记录

**Files:**
- Modify: `docs/agents/claude-chain.md`

- [ ] **Step 1: 追加修复记录**

在 `docs/agents/claude-chain.md` 的末尾追加一节：

```markdown

## 2026-04-22 — history resume 绑定到 sess.task_id + stream 指示器读 per-task bucket

### 现象

- 选 Claude 历史 session 后点 Connect，daemon 成功 launch，但前端按钮一直停留在 Connect，`claudeOnlineForTask` 永远为 false。
- Claude agent 在工作但 UI 没有流式指示器（thinking / writing / tool 都看不到）。

### 根因

1. `DaemonCmd::ResumeSession` 的 Claude 分支用 `active_task_id.unwrap_or_default()` 作为 `resume_task_id`，当前无 active task 时变成空串，有 active task 但和 session 原始 task 不同时会绑到错误 task。Codex 同一路径已经使用 `sess.task_id.clone()`。
2. `ClaudeStreamIndicator` / `CodexStreamIndicator` 只订阅 singleton mirror，而 mirror 只在 `activeId === taskId` 时才更新；per-task bucket 明明有 stream，indicator 也会失声。
3. 对照排查后确认：Codex backend history resume 没有同类 bug，但 `CodexPanel.handleConnect` 也存在 stale `activeTask?.taskId` 闭包。

### 修复

- Claude resume 改用 `sess.task_id.clone()` 与 Codex 对齐。
- 新增 `makeActiveClaudeStreamSelector` / `makeActiveCodexStreamSelector`，indicator 组件改读 per-task bucket。
- `ClaudePanel.doLaunch` / `CodexPanel.handleConnect` 的 `useCallback` 依赖补齐 `activeTask?.taskId`。

### 验证

- `cargo test -p dimweave` 无新增失败
- `bun test src/stores/bridge-store/ src/components/MessagePanel/` 无新增失败
- 手工：多 task 下 stream indicator 只跟随 active task；Claude 历史 session Connect 后按钮翻为 Disconnect 并切到 session 原生 task
```

- [ ] **Step 2: 提交**

```bash
git add docs/agents/claude-chain.md
git commit -m "$(cat <<'EOF'
docs(claude-chain): record stream-indicator and history-resume fix

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7 — CM 回填与最终验证

**Files:**
- Modify: `docs/superpowers/plans/2026-04-22-claude-stream-fix-and-history-resume.md`

- [ ] **Step 1: 跑完整构建通路**

```bash
cd /Users/jason/floder/agent-bridge && cargo test -p dimweave
cd /Users/jason/floder/agent-bridge && bun test
cd /Users/jason/floder/agent-bridge && bun x tsc --noEmit -p tsconfig.app.json
cd /Users/jason/floder/agent-bridge && bun run build
```

记录实际 pass / fail 数。预先已知的 5 个 pre-existing 失败（见 `2026-04-21-production-load-hardening.md`）可放行，**不允许引入新的失败**。

- [ ] **Step 2: 回填 commit hash**

在本 plan 末尾（Task 7 之后）追加 CM 回填区：

```markdown
## CM 回填区

- `<hash1>` — `fix(daemon): Claude resume binds to sess.task_id, matching Codex` — Task 1
- `<hash2>` — `feat(bridge-store): add per-task stream selectors with stable defaults` — Task 2
- `<hash3>` — `fix(stream): ClaudeStreamIndicator reads per-task bucket directly` — Task 3
- `<hash4>` — `fix(stream): CodexStreamIndicator reads per-task bucket directly` — Task 4
- `<hash5>` — `fix(panels): include activeTask.taskId in Claude and Codex launch callbacks` — Task 5
- `<hash6>` — `docs(claude-chain): record stream-indicator and history-resume fix` — Task 6
```

用 `git log --oneline -20` 拿真实 hash 填进去。

- [ ] **Step 3: 手工 E2E（dev 模式）**

```bash
cd /Users/jason/floder/agent-bridge && bun run tauri dev
```

在 UI 里：

1. 启动后拉起两个 task（task A + task B），两边都挂 Claude lead。
2. Task A 发一条消息让 Claude 开始思考。Task A 视图要看到流式指示器（thinking → writing → result）。切到 Task B 时指示器要立刻消失；切回 Task A 时要即时恢复。
3. 在 Task A 里选一条 Claude 历史 session，点 Connect。按钮要在 daemon launch 完成后翻成 Disconnect，UI 自动跳到 session 原生 task（如果不是当前 task）。
4. 观察 `bun run tauri dev` 的日志：`[Claude Trace] chain=event_dispatch` 的 `task_id` 要等于 session 原始 task，不应出现空串。
5. 切换到另一个 task 后，分别验证 Codex 的新建连接和 `resumeExternal` 路径不会再打到上一次 active task。

- [ ] **Step 4: 提交 CM 回填**

```bash
git add docs/superpowers/plans/2026-04-22-claude-stream-fix-and-history-resume.md
git commit -m "$(cat <<'EOF'
docs: backfill CM for 2026-04-22 claude stream and history resume plan

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## 验收

- `cargo test -p dimweave` 全量失败数不增加。
- `bun test src/stores/bridge-store/selectors.test.ts` 5/5 通过。
- `bun test` 总体失败数 ≤ baseline（`2026-04-21-production-load-hardening.md` 记录的 5 个 pre-existing）。
- `bun x tsc --noEmit` 无新增错误。
- `bun run build` 成功。
- 手工 E2E：
  1. 多 task 并发下流式指示器只在 active task 出现。
  2. Task 切换时指示器能即时跟随，不会卡在旧任务，也不会无故空白。
  3. 选 Claude 历史 session 点 Connect：launch 成功后按钮翻成 Disconnect，并切到 session 原生 task。
  4. daemon 日志里 Claude 事件 `task_id` 等于 session 原始 task，不再出现空串。
  5. Codex 在切 task 后的首次新建连接 / `resumeExternal` 也不会命中旧 taskId。

## 执行备注

- 实际执行时额外发现一个计划外但必要的漏点：`src/components/MessagePanel/MessageList.tsx` 外层仍在用 singleton mirror 决定 Claude draft 行和 Codex footer 是否渲染。仅修改 `ClaudeStreamIndicator` / `CodexStreamIndicator` 本身并不足以恢复 UI，因此实现里一并把 `MessageList` 切到了 active-task selector，并补了对应回归测试。
- `bun x tsc --noEmit -p tsconfig.app.json` 在当前仓库状态下存在大量 pre-existing 错误，主要集中在 `bun:test` 类型、若干旧测试夹具、`import.meta.hot` 以及和本次改动无关的旧组件类型问题；这一步不能作为本次修复的 clean gate。
- `bun test` 当前全量结果为 `486 pass / 6 fail`。失败项均与本次改动无直接关系，分别是：
  - `tests/vite-config.test.ts`
  - `tests/message-panel-view-model.test.ts`
  - `tests/codex-launch-config.test.ts`
  - `src/components/ClaudePanel/launch-request.test.ts`（2 条）
  - `src/components/ReplyInput/index.test.tsx`

## CM 回填区

- `fb1ed2a` — `fix(daemon): Claude resume binds to sess.task_id, matching Codex` — Task 1
- `775a706` — `feat(bridge-store): add per-task stream selectors with stable defaults` — Task 2
- `dc68a3f` — `fix(stream): read active-task buckets for message panel indicators` — Tasks 3-4 + `MessageList` 补漏
- `03b4d07` — `fix(panels): include activeTask.taskId in launch callbacks` — Task 5
- `ff3af09` — `docs(claude-chain): record stream-indicator and history-resume fix` — Task 6

## 实际验证结果

- `cargo test -p dimweave` — ✅ `755 passed, 0 failed`
- `cargo test -p dimweave claude_resume_uses_session_task` — ✅ `2 passed, 0 failed`
- `cargo test -p dimweave state_task_snapshot_tests` — ✅ `15 passed, 0 failed`
- `cargo test -p dimweave claude_tests` — ✅ `18 passed, 0 failed`
- `bun test src/stores/bridge-store/selectors.test.ts` — ✅ `5 passed, 0 failed`
- `bun test src/components/MessagePanel/ src/stores/bridge-store/` — ✅ `64 passed, 0 failed`
- `bun test src/components/ClaudePanel/connect-state.test.ts src/components/AgentStatus/codex-launch-config.test.ts src/components/TaskPanel/index.test.tsx` — ✅ `12 passed, 0 failed`
- `bun test` — ⚠️ `486 passed, 6 failed`（均为当前仓库已有无关失败项）
- `bun x tsc --noEmit -p tsconfig.app.json` — ⚠️ pre-existing failures remain
- `bun run build` — ✅ success
