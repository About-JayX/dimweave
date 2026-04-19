# Auth / Routing / Task-isolation Code-Review Followups

## 目标

把 Provider Auth + Agent Lifecycle + 多 task 消息隔离这条链路的 code review 里
**实锤的 bug** 全部修掉。

- **C1** routing.rs 的 `claude_sender_ok` 用 singleton `s.codex_role` 做
  sender gating，multi-task 下被覆写会丢消息
- **C2** 临时 CODEX_HOME 在 handshake 失败 / session 被取代 / spawn 失败的
  bail 分支里不清理，`/tmp/dimweave-codex-apikey-<pid>-<sid>/` 泄漏
- **H2** TaskPanel edit 流程串行 stopAgent + launch，任一失败整个流产，外层
  catch 吞掉细节；结果是部分 agent 已切换、部分没切
- **M3** 前端 `filterMessagesByTaskId` 严格丢弃所有无 taskId 消息，和后端
  routing 允许 "legacy/system messages without task context" 的语义打架，
  合法全局系统消息会被吃掉
- **H1** 后端 `upsert_provider_auth` 没在 subscription 模式下强制清空 api_key
  字段（UI 已经 scrub 了，但缺防御层）

已评估不修：
- **H3** `SaveProviderAuth` / `Launch*` 都走同一个 daemon command loop 串行
  处理，没有实际竞态窗口；auth_version 属于 defense-in-depth
- **H4** 短 key (< 20 字符) 逻辑本身成立，只是缺一条单测
- Other L 级条目（symlink warn 日志、MSG_SEQ global atomic）不影响行为

## 修复顺序 + 原则

1. **C1 → C2 → H2 → M3 → H1** 按 blast radius 排
2. **每个 bug 独立 commit**，好 bisect
3. **绝不碰周边链路**：
   - singleton `s.codex_role` / `s.claude_role` 继续存在（其它 caller 可能仍
     用，legacy fallback），C1 只改 `claude_sender_ok` 的计算方式
   - M3 不动后端 stamping，只放宽前端过滤
   - C2 的 RAII 化是 codex/mod.rs 内部细节，不影响 `CodexHandle` 对外 API

## Step 1 — C1 sender gate 迁到 task-first

**File**: `src-tauri/src/daemon/routing.rs:265-266`

```rust
// 改前
let claude_sender_ok =
    msg.is_from_user() || msg.is_from_system() || msg.source_role() == s.codex_role;

// 改后
let claude_sender_ok = msg.is_from_user()
    || msg.is_from_system()
    || match msg.task_id.as_deref() {
        Some(tid) => s.task_graph.agents_for_task(tid).iter().any(|a|
            a.provider == Provider::Codex && a.role == msg.source_role()
        ),
        None => msg.source_role() == s.codex_role,  // legacy fallback
    };
```

**测试**：新增回归测试，场景 = 两个 task 各自有 Codex coder + Claude lead，
两个 task 并发运行时 Task A 的 coder → Task A 的 claude lead 不会被
Task B 覆写的 singleton 误 gate。

## Step 2 — C2 TempCodexHome RAII + startup sweep

**Files**:
- `src-tauri/src/daemon/codex/mod.rs` — 新增 `struct TempCodexHome(PathBuf)`
  + `impl Drop`；`CodexHandle` 持 `Option<TempCodexHome>`；所有 bail 分支
  自动清理
- 新 free fn `prune_orphan_api_key_homes()` — 在 daemon startup 扫
  `/tmp/dimweave-codex-apikey-*`，PID 非当前的全清

**不改**：`CodexHandle::stop()` 的外部 API、`build_api_key_codex_home`
返回 PathBuf 的契约由 Drop 承担，调用点不需要再显式调 cleanup。

## Step 3 — H2 TaskPanel edit 流程 allSettled + error surface

**File**: `src/components/TaskPanel/index.tsx::handleDialogSubmit`

```ts
// 改前：for await 串行，任一 throw 中断
for (const a of savedAgents) {
  if (a.agentId) await stopAgent(a.agentId);
}
await launchProviders(task.taskId, task.taskWorktreeRoot, savedAgents);

// 改后：两阶段 allSettled，错误不吞
const stopResults = await Promise.allSettled(
  savedAgents.filter(a => a.agentId).map(a => stopAgent(a.agentId!))
);
const launchResult = await launchProviders(...).catch(e => ({ error: e }));
const errors = [
  ...stopResults.filter(r => r.status === "rejected")
    .map(r => `stop: ${(r as PromiseRejectedResult).reason}`),
  ...(launchResult && "error" in launchResult ? [`launch: ${launchResult.error}`] : []),
];
if (errors.length) {
  // surface via system log event; 不 swallow
}
```

**不改**：create 流程（无 stop 阶段，不需要）。

## Step 4 — M3 filterMessagesByTaskId 放行 system 消息

**File**: `src/stores/bridge-store/selectors.ts`

```ts
// 改后
return messages.filter((m) =>
  m.taskId === taskId || m.source?.kind === "system"
);
```

**同时**：review `src-tauri/src/daemon/codex/session_event.rs` 里所有
`emit_agent_message` + `source = "codex"` / `"system"` 的分类是否合理。
per-task 诊断必须 stamp（已做）；真·全局系统消息应 `source = "system"`
（已有）。

## Step 5 — H1 upsert_provider_auth subscription 强制清空

**File**: `src-tauri/src/daemon/task_graph/store.rs::upsert_provider_auth`

```rust
pub fn upsert_provider_auth(&mut self, mut cfg: ProviderAuthConfig) {
    if cfg.active_mode.as_deref() == Some("subscription") {
        cfg.api_key = None;
        cfg.base_url = None;
        cfg.wire_api = None;
        cfg.auth_mode = None;
        cfg.provider_name = None;
    }
    cfg.updated_at = chrono::Utc::now().timestamp_millis() as u64;
    self.provider_auth.insert(cfg.provider.clone(), cfg);
}
```

加一条单测：`upsert_subscription_scrubs_credential_fields`。

## 验证

- `cargo test -p dimweave` —  700+ 全绿 + 新增测试
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：
  1. 启动两个 task 各 `Codex coder + Claude lead`，并发跑，**Task A 的 coder
     → Claude lead 不丢消息**（C1 验证）
  2. 触发 Codex handshake 失败（如 port 被占用），检查
     `/tmp/dimweave-codex-apikey-*` **无泄漏**（C2 验证）
  3. edit 流程模拟 stopAgent 失败，另一个 agent 的 launch **仍然进行**
     （H2 验证）
  4. 切 task 切回来，系统级消息（agent offline 等）**都能看到**（M3 验证）
  5. dialog 改成 subscription 保存后，查 SQLite **api_key 字段为 NULL**
     （H1 验证）

## Commit 规划

1. `fix(routing): sender gate task-first so multi-task doesn't gate legit messages`
2. `fix(codex-auth): RAII temp CODEX_HOME + startup sweep for orphans`
3. `fix(ui): TaskPanel edit allSettled + surface errors`
4. `fix(messages): let source=system messages cross task boundaries`
5. `fix(provider-auth): scrub credential fields when saving subscription mode`

## CM 回填区

- `981fc5f0` — `docs: plan auth/routing code-review followups (C1/C2/H2/M3/H1)` — 本 plan 文档
- `56b50f0c` — `fix(routing): sender gate task-first so multi-task doesn't gate legit messages` — C1 修复 + `sender_gate_survives_codex_role_singleton_flip` 回归测试
- `250a6998` — `fix(codex-auth): RAII temp CODEX_HOME + startup sweep for orphans` — C2 `TempCodexHome` RAII 包装 + daemon 启动时 PID-gated sweep `/tmp/dimweave-codex-apikey-*`
- `c10493a4` — `fix(ui): TaskPanel edit allSettled + surface errors instead of swallowing` — H2 `handleDialogSubmit` 两段 try + stop phase `Promise.allSettled` + 每个 rejected stop 记 console.error
- `1eaca85b` — `fix(messages): let source=system messages cross task boundaries` — M3 `filterMessagesByTaskId` 放行 `source.kind === "system"` + 文档注释说明语义
- `6e4e8c68` — `fix(provider-auth): scrub credential fields when saving subscription mode` — H1 `upsert_provider_auth` 在 subscription 模式下强制清空 credential 字段 + `upsert_subscription_scrubs_credential_fields` 单测

### 验证

- `cargo test` — 741 passed（+2 新测试：C1 race + H1 scrub）
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK

### 明确不修（已评估）

- **H3** — `SaveProviderAuth` / `Launch*` 都走同一个 `cmd_rx` 串行消费（`daemon/mod.rs:443`），实际无竞态窗口，`auth_version` 属于纵深防御
- **H4** — 短 key (< 20 字符) 逻辑在 `process.rs::apply_provider_auth` 已经 chars().rev().take(20).rev() 正确兜底；只是单测没覆盖短 key 场景
- **L1/L2/L3/L4** — 样式 / 日志补强 / 文档类建议，非 blocking
