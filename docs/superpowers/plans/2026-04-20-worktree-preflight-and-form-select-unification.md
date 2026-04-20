# 2026-04-20 — Worktree Preflight + Form-Select Unification

## Context

继 `2026-04-20-persistence-restart-recovery.md` 之后发现两类遗留：

1. **Stale worktree cwd 导致 spawn 失败**：用户手动清理 `.worktrees/` 后，
   `task.task_worktree_root` 仍指向被删除路径。`Command::spawn().current_dir(cwd)`
   在 cwd 不存在时 `execve` 返回 `ENOENT`，错误被 `emit_system_log(error)`
   吞掉，开发者看到的只是 `[Codex] using binary` 再无后文，排查极其困难。
2. **下拉样式分裂**：`ProviderAuthDialog` 原本用 native `<select>`（native
   popup 浮在屏幕左上角，和 input 高度不齐），`TaskSetupDialog` 用
   `CyberSelect` 默认 variant 的小型 pill，两处视觉完全不一致。用户反复
   提出"应该统一"。

顺带清理一条小 bug：`DIMWEAVE_CODEX_DEBUG` 未设时把 `RUST_LOG=""` 硬注入
Codex 子进程，Codex 自身 tracing init 在该空串下拒启，banner 不打，WS
不绑——和第 1 条一起让问题看起来像"Codex 完全没启动"。

## 落地内容

### Step 1 — Codex cwd preflight + typed error
- 文件：`src-tauri/src/daemon/mod.rs`（`LaunchCodex` / `LaunchClaudeSdk` handler）
- spawn 前 `std::path::Path::new(&cwd).is_dir()` 检查。不存在时 `reply.send(Err(format!("WORKTREE_MISSING:{resolved_task_id}:{cwd}")))`，
  `continue` 跳过 spawn。
- 前端可按 `WORKTREE_MISSING:<taskId>:<path>` 前缀识别，无需猜解 libc 错误文案。

### Step 2 — TaskPanel worktree-missing confirm dialog
- 文件：`src/components/TaskPanel/index.tsx`
- 新增 local state `worktreeMissing: { taskId, path } | null`。
- `handleDialogSubmit` 的 `launchProviders` catch 里 regex 匹配
  `/WORKTREE_MISSING:([^:]+):(.+)$/`，命中则 set state 弹 `ConfirmDialog`：
  > The worktree at {path} no longer exists. Delete this task and all its data?
- 确认 → `deleteTask(taskId)` 走既有级联路径；取消 → 对话框关闭。
- 第二个 `<ConfirmDialog open={!!worktreeMissing}>` 与既有"用户主动删除"对话框
  并列，共用组件不共用 state。

### Step 3 — Codex debug env fix
- 文件：`src-tauri/src/daemon/codex/lifecycle.rs`
- 之前把 `cmd.env("RUST_LOG", ...)` 无条件注入（未启用 debug 时值为 `""`），
  导致 Codex 自带 tracing init 在空 `RUST_LOG` 下异常退出。
- 改为"仅当用户显式 `DIMWEAVE_CODEX_DEBUG=1` 且父环境未设 `RUST_LOG`"时
  才注入 `"codex=debug,reqwest=debug,hyper=info"`。否则继承默认行为。

### Step 4 — Codex spawn FATAL diagnostics
- 文件：`src-tauri/src/daemon/codex/mod.rs`
- `launch()` poll 循环内的 `anyhow::bail!` 前加 `eprintln!`：
  - 子进程提前退出 → `[Codex] FATAL: subprocess exited prematurely on port=... status=...`
  - 10s 内未绑定端口 → `[Codex] FATAL: app-server did not bind port=... within 10s`
- 日志同时经 `emit_system_log(error)` 给 GUI，并经 stderr 给 monitor 抓。

### Step 5 — CyberSelect 新增 `form` variant
- 文件：`src/components/ui/cyber-select.tsx`
- 类型：`variant?: "default" | "history" | "form"`。
- Trigger 样式：`w-full rounded-md border border-border/40 bg-background
  px-2 py-1.5 text-[11px]` —— 完全对齐同级 `<input>` 的视觉尺寸。
- Panel 位置：`getCyberSelectMenuPanelClassName("form")` 返回
  `"left-0 top-full mt-1 w-full max-h-52 rounded-md p-1"`，贴 trigger
  下方、等宽。（native select popup 会浮到屏幕左上角，换 CyberSelect
  后统一按相对定位。）
- Button 开关态配色适配 form 场景：`border-border/40 bg-background`，
  与 input 默认状态一致。

### Step 6 — ProviderAuthDialog 统一用 CyberSelect(form)
- 文件：`src/components/ToolsPanel/ProviderAuthDialog.tsx`
- Wire API 和 Auth header 两个 native `<select>` 全部替换为
  `<CyberSelect variant="form" ... />`。
- options 改成 `CyberSelectOption[]`（`{ value, label }`）。

### Step 7 — TaskSetupDialog 所有下拉改 form 布局
- 文件：`src/components/TaskPanel/TaskSetupDialog.tsx`
- 5 个下拉（Provider / Role / Model / Effort / Session）从
  `flex items-center justify-between` + 右侧 pill 小型 CyberSelect，
  改成 `space-y-0.5` + 上方 label + 下方 full-width `variant="form"`。
- Session 列表也一并切到 form，弃用之前的 `compact history` 变体；
  长 thread id / session id 走 CyberSelect 默认 menu 的 `truncate`
  路径，仍能读。

## 关键文件清单

| 文件 | 角色 |
|---|---|
| `src-tauri/src/daemon/mod.rs` | cwd preflight + WORKTREE_MISSING typed error + FATAL diagnostics |
| `src-tauri/src/daemon/codex/mod.rs` | launch() 的 eprintln diagnostics |
| `src-tauri/src/daemon/codex/lifecycle.rs` | RUST_LOG env 条件注入 |
| `src/components/TaskPanel/index.tsx` | worktree-missing catch + confirm dialog |
| `src/components/TaskPanel/TaskSetupDialog.tsx` | 5 下拉切 form variant + 垂直 label |
| `src/components/ToolsPanel/ProviderAuthDialog.tsx` | 两个 select 切 CyberSelect(form) |
| `src/components/ui/cyber-select.tsx` | `form` variant 定义（trigger + panel 样式） |

## 验证

- `cargo check -p dimweave` — clean
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新 TS 错误（既有 pre-existing
  bun:test + ClaudeLaunchRequest/InvokeArgs 不变）
- 手工 E2E：
  1. 删除 `.worktrees/` 后点 Save & Connect → 弹"worktree missing, delete task?"
     → 确认后 task 级联清理，UI 回到空任务列表
  2. `DIMWEAVE_CODEX_DEBUG=1 bun run tauri dev` → Codex 子进程 stderr 打
     reqwest/hyper debug 行
  3. `bun run tauri dev`（不带 debug）→ Codex banner 正常打印，handshake 通过
  4. Tools → Accounts → Claude / Codex 的 Advanced 段：Wire API / Auth
     header 下拉形态与同段 `<input>` 完全对齐；弹出面板位于 trigger 下方、
     等宽
  5. Edit Task → 5 个下拉从 pill 改为 input-like 垂直布局，与 Accounts
     dialog 视觉一致

## 已确认设计决策

- **手动 confirm 删除** stale task（不自动删）— 对应 persistence 计划里
  "auth 变更手动重连" 的安全基调，避免不可撤销操作。
- **CyberSelect 三 variant 共存**：`default`（inline pill，状态栏用）
  `history`（带 middleEllipsis 的列表弹窗，provider 历史用）
  `form`（input-like 全宽，表单用）。不再允许组件外再写 native select。
- **WORKTREE_MISSING 用字符串前缀**：简单、不用新 Tauri 枚举；前端
  regex 一次。如未来错误码膨胀再抽类型。

## 明确不做

- ❌ 给所有 ClaudePanel / CodexPanel 独立 Connect 按钮也加 worktree-missing
  dialog（目前只在 TaskPanel Save & Connect 走）—— 单例 Connect 按钮通常
  指向 active task，fallback 逻辑与 DeleteTask 纠缠，本轮不展开。
- ❌ 给 `form` variant 新增 description 行（当前 default menu 已支持）。
- ❌ 把 CyberSelect history variant 迁移到 form —— 那条路径有
  `middleEllipsis` + `HistoryMenuOption` 特化，本轮不合并。

## CM (Configuration Management)

### Commit
- **Hash**: `9bbc880`
- **Subject**: `feat(ui,daemon): worktree preflight confirm + CyberSelect form variant`
- **Scope**: 7 files. daemon-side preflight + diagnostics + RUST_LOG guard，
  前端 `form` variant 定义 + 两个对话框全面迁移。
