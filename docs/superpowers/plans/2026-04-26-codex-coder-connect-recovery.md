# 2026-04-26 Codex Coder Connect Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复新 task 中 coder / Codex 连接失败、worker 无 lead 时结果不可见，以及多 task stream 状态写桶的一致性问题。

**Architecture:** 后端把 Codex 端口分配收敛到一个可避让 OS listener 与 daemon 端口的 pool，并把 worker -> lead 的回退判断改成基于“lead agent 在线可达”而不是仅基于“lead 角色存在”。前端 stream batching 继续使用 per-task bucket，flush 阶段必须写入确定 task bucket，避免 active task 切换造成状态串桶。计划按可独立回滚的 task 拆分，每个 task 都有对应测试和 CM 记录。

**Tech Stack:** Rust Tokio daemon, Tauri 2, React 19, Zustand bridge-store/task-store, `cargo test`, `bun test`

---

## Review 结论

### Important: 无 lead 回退只检查角色存在，没有检查在线状态

`src-tauri/src/daemon/routing.rs` 的 `retarget_worker_reply_to_user_when_no_lead()` 当前逻辑：

```rust
let agents = s.task_graph.agents_for_task(task_id);
if agents.is_empty() || agents.iter().any(|agent| agent.role == "lead") {
    return msg;
}
```

这会导致 task 里只要存在 lead agent 记录，即使 lead 离线，coder 回复 `target=lead` 也不会被回退到 user。结果会被路由层当成发给 lead 的消息继续解析，常见表现是 buffering 或不可见，和 prompt 里新增的“no lead is available or online -> reply user”不一致。

### Medium: 端口避让已覆盖当前 4502 症状，但仍可能被 stale Codex 进程耗尽 pool

`CodexPortPool::reserve_skipping()` 会跳过 OS 已占用端口和 daemon 端口，能避免 `4500/4501` 旧 Codex、`4502` daemon 同时存在时仍硬分配到 `4502`。但被跳过的旧 `codex app-server` 不会触发 `ensure_port_available()` 的 `kill_port_holder()` 清理；连续 dev restart 后，4500-4507 都可能被孤儿进程占满，最终还是 `no Codex port available in pool`。

### Low: `route_message_with_display()` 和 `route_message_inner_with_meta()` 双重执行同一个 retarget

当前不会造成二次改写，因为第一次改成 user 后第二次会因 target 不是 lead 返回原消息。但这会让后续维护者误以为两个路径都必须单独处理。后续修复在线 lead 判断时，建议保留 inner 层作为唯一入口，外层只消费 `outcome` 和持久化副作用。

## 文件结构

| 文件 | 角色 |
|---|---|
| `src-tauri/src/daemon/codex/port_pool.rs` | Codex 端口 lease、daemon 端口排除、OS listener 跳过 |
| `src-tauri/src/daemon/codex/port_pool_tests.rs` | 端口池避让、释放、stale launch 回归测试 |
| `src-tauri/src/daemon/mod.rs` | Codex launch/resume/attach 分配端口并启动 app-server |
| `src-tauri/src/daemon/routing.rs` | 消息路由与 worker 无 lead 回退入口 |
| `src-tauri/src/daemon/routing_dispatch.rs` | GUI 持久化、routing side effects、released message 递归处理 |
| `src-tauri/src/daemon/routing_behavior_tests.rs` | worker 输出可见性与 routing 行为回归测试 |
| `src-tauri/src/daemon/routing_user_input.rs` | user auto target 解析，按 task agent 在线状态过滤 |
| `src-tauri/src/daemon/routing_user_target_tests.rs` | auto target 同 provider 多 agent 在线性回归测试 |
| `src-tauri/src/daemon/state_runtime.rs` | task-local per-agent runtime slot 与 provider connection 查询 |
| `src-tauri/src/daemon/state_snapshot.rs` | task provider summary / online agents snapshot |
| `src-tauri/src/daemon/state_snapshot_tests.rs` | same-provider lead/coder summary 回归测试 |
| `src-tauri/src/daemon/role_config/*.rs` | role/prompt 协议：无 lead 时 worker 可向 user 回报 |
| `src/stores/bridge-store/listener-setup.ts` | stream event 分桶、active task mirror、pending flush 调度 |
| `src/stores/bridge-store/stream-batching.ts` | pending Claude/Codex stream batch reduce 到 global 或 task bucket |
| `src/stores/bridge-store/listener-setup.test.ts` | stream batching 写入 active task bucket 的前端回归测试 |
| `docs/superpowers/plans/2026-04-26-codex-coder-connect-recovery.md` | 本 plan 与 CM 回填记录 |

---

## Task 1: Codex 端口避让与启动弹性

**Files:**
- Modify: `src-tauri/src/daemon/codex/port_pool.rs`
- Modify: `src-tauri/src/daemon/codex/port_pool_tests.rs`
- Modify: `src-tauri/src/daemon/mod.rs`

- [x] **Step 1: 写端口避让回归测试**

覆盖 daemon 端口排除和 OS 已占用端口跳过：

```rust
#[test]
fn codex_port_pool_skips_excluded_daemon_port() {
    let mut pool = CodexPortPool::new_with_excluded_ports(4500, [4502]);
    assert_eq!(pool.reserve("task_a", 1), Some(4500));
    assert_eq!(pool.reserve("task_b", 2), Some(4501));
    assert_eq!(pool.reserve("task_c", 3), Some(4503));
    assert!(!pool.leased_ports().contains(&4502));
}
```

- [x] **Step 2: 实现 pool 级排除和 OS listener 跳过**

`CodexPortPool` 增加 `excluded_ports`，daemon 初始化时把 `daemon_port` 放入排除集合；launch / resume / attach 都改用 `reserve_skipping(..., codex_port_unavailable)`。

- [x] **Step 3: 验证端口池测试**

Run:

```bash
cd /Users/jay/floder/dimweave/src-tauri
cargo test -p dimweave codex_port_pool
```

Observed: `17 passed; 0 failed`。有既存 warning，与本 task 无关。

- [x] **Step 4: 补强启动 race / stale process 行为**

新增一个 launch-level retry：如果 `codex::start()` 或 `codex::resume()` 返回 `Port N still in use...`，释放当前 lease，重新 `reserve_skipping()` 下一个端口并重试一次。这样能覆盖 reserve 与 child bind 之间的 TOCTOU race，也能在 `kill_port_holder()` 未能立即释放时自动后推。

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/daemon/codex/port_pool.rs \
  src-tauri/src/daemon/codex/port_pool_tests.rs \
  src-tauri/src/daemon/mod.rs
git commit -m "fix(daemon): skip occupied Codex app-server ports"
```

Observed commit: `78cbd10`.

## Task 2: Worker 无在线 lead 时结果回退到 user

**Files:**
- Modify: `src-tauri/src/daemon/routing.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`
- Modify: `src-tauri/src/daemon/routing_behavior_tests.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- Modify: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`

- [x] **Step 1: 覆盖“task 没有 lead agent”场景**

`worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead` 已覆盖 coder 在无 lead task 中回复 lead 时要进入 GUI，而不是被 buffer。

- [x] **Step 2: 补一个“lead 存在但离线”的失败测试**

在 `src-tauri/src/daemon/routing_behavior_tests.rs` 增加测试：

```rust
#[tokio::test]
async fn worker_reply_to_offline_lead_surfaces_to_gui() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let task_id = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "Offline Lead");
        let _lead = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");
        let coder = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Codex, "coder");
        s.init_task_runtime(&task.task_id, "/ws".into());
        let (tx, _) = tokio::sync::mpsc::channel(1);
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_codex_slot(&coder.agent_id, 4500)
            .inject_tx = Some(tx);
        task.task_id
    };

    let msg = BridgeMessage {
        id: "coder-offline-lead-1".into(),
        source: MessageSource::Agent {
            agent_id: "codex-coder-1".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: Some("codex".into()),
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "done".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: Some(task_id),
        session_id: None,
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::ToGui));
    assert!(state.read().await.buffered_messages.is_empty());
}
```

- [x] **Step 3: 修改 retarget 判断为“没有在线 lead”**

把 `retarget_worker_reply_to_user_when_no_lead()` 的存在性检查改为在线性检查：

```rust
let has_online_lead = agents.iter().any(|agent| {
    agent.role == "lead"
        && state_runtime_for_agent_is_online(s, task_id, &agent.agent_id, agent.provider)
});
if has_online_lead {
    return msg;
}
```

实现时优先复用现有 `DaemonState::is_task_agent_online_by_id(task_id, agent_id, runtime)`，用 `provider_runtime(agent.provider)` 或等价 local match 得到 `"claude"` / `"codex"`。

- [x] **Step 4: 去掉重复 retarget 入口**

保留 `route_message_inner_with_meta()` 里的 retarget，删除 `route_message_with_display()` 开头的重复调用。确认 `route_message()` 和 `route_message_inner()` 两类入口都仍覆盖。

- [x] **Step 5: 验证 routing 与 prompt 测试**

Run:

```bash
cd /Users/jay/floder/dimweave/src-tauri
cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead
cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui
cargo test -p dimweave coder_prompt_falls_back_to_user_when_no_lead_available
```

Expected: 三组都通过，且无新增失败。

Observed:

- `cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead` -> `1 passed; 0 failed`
- `cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui` -> `1 passed; 0 failed`
- `cargo test -p dimweave coder_prompt_falls_back_to_user_when_no_lead_available` -> `2 passed; 0 failed`

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/daemon/routing.rs \
  src-tauri/src/daemon/routing_dispatch.rs \
  src-tauri/src/daemon/routing_behavior_tests.rs \
  src-tauri/src/daemon/role_config/claude_prompt.rs \
  src-tauri/src/daemon/role_config/claude_prompt_tests.rs \
  src-tauri/src/daemon/role_config/role_protocol.rs \
  src-tauri/src/daemon/role_config/roles.rs \
  src-tauri/src/daemon/role_config/roles_tests.rs
git commit -m "fix(routing): surface worker replies when no lead is online"
```

Observed commit: `c20f9b1`.

## Task 3: Same-provider 多 agent 的 task summary / auto target 收敛

**Files:**
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/routing_user_target_tests.rs`
- Modify: `src-tauri/src/daemon/state_runtime.rs`
- Modify: `src-tauri/src/daemon/state_snapshot.rs`
- Modify: `src-tauri/src/daemon/state_snapshot_tests.rs`

- [x] **Step 1: auto target 改成按 agent_id 在线状态过滤**

`resolve_user_targets_for_task()` 已从 provider-level `is_task_agent_online(task_id, runtime)` 改为 agent-level `is_task_agent_online_by_id(task_id, &m.agent_id, m.runtime)`。

- [x] **Step 2: provider summary 改成按 agent_id 读取 connection**

`task_provider_connection_for_agent()` 已用于 same-provider lead/coder summary，避免 Codex lead 和 Codex coder 抢同一个 provider-level connection。

- [x] **Step 3: 验证 same-provider 测试**

Run:

```bash
cd /Users/jay/floder/dimweave/src-tauri
cargo test -p dimweave agent_id_routing_auto_same_provider
cargo test -p dimweave task_provider_summary_same_provider_uses_each_agent_connection
```

Observed: `agent_id_routing_auto_same_provider` 两条测试通过；`task_provider_summary_same_provider_uses_each_agent_connection` 通过。

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/routing_user_input.rs \
  src-tauri/src/daemon/routing_user_target_tests.rs \
  src-tauri/src/daemon/state_runtime.rs \
  src-tauri/src/daemon/state_snapshot.rs \
  src-tauri/src/daemon/state_snapshot_tests.rs
git commit -m "fix(task-runtime): resolve same-provider agents by agent id"
```

Observed commit: `ae7241d`.

## Task 4: Stream batching 写入 task bucket

**Files:**
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/stream-batching.ts`
- Modify: `src/stores/bridge-store/listener-setup.test.ts`

- [x] **Step 1: batching flush 接收 active task id**

`flushPendingStreamUpdates(state, pending, activeTaskId)` 已支持把 Claude preview / Codex delta flush 到 active task bucket，同时更新 singleton mirror。

- [x] **Step 2: 覆盖 Codex delta 写入 active task bucket**

`flushes queued Codex delta into the active task stream bucket` 已验证 active task 的 `codexStreamsByTask[taskId].currentDelta` 和 mirror 同步。

- [x] **Step 3: 验证前端聚焦测试**

Run:

```bash
cd /Users/jay/floder/dimweave
bun test src/stores/bridge-store/listener-setup.test.ts
```

Observed: `14 pass; 0 fail`。

- [x] **Step 4: 补 task switch pending 清理测试**

新增一个 listener-level 或 pure helper 测试，覆盖 active task 切换时 pending stream 被清空，避免 32ms flush timer 把旧 task 的 pending 写进新 active task bucket。若测试夹具难以直接覆盖 subscribe 行为，优先把 pending flush bucket id 固化到 pending 结构里：

```ts
type PendingStreamUpdates = {
  bucketTaskId: string | null;
  claudePreviewText: string;
  codexActivity: string | null;
  codexReasoning: string | null;
  codexDelta: string | null;
  codexCommandOutput: string;
};
```

然后 `queue*` 时记录 task id，`flushPendingStreamUpdates()` 用 `pending.bucketTaskId` 而不是读取 flush 时的 active task。

- [x] **Step 5: Commit**

```bash
git add src/stores/bridge-store/listener-setup.ts \
  src/stores/bridge-store/stream-batching.ts \
  src/stores/bridge-store/listener-setup.test.ts
git commit -m "fix(stream): flush pending updates into task buckets"
```

Observed commit: `df017f5`.

## Task 5: 最终验证与 CM 回填

**Files:**
- Modify: `docs/superpowers/plans/2026-04-26-codex-coder-connect-recovery.md`

- [x] **Step 1: 跑聚焦验证**

Run:

```bash
cd /Users/jay/floder/dimweave/src-tauri
cargo test -p dimweave codex_port_pool
cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead
cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui
cargo test -p dimweave agent_id_routing_auto_same_provider
cargo test -p dimweave task_provider_summary_same_provider_uses_each_agent_connection

cd /Users/jay/floder/dimweave
bun test src/stores/bridge-store/listener-setup.test.ts
git diff --check
```

Expected: 所有聚焦测试通过，`git diff --check` 无输出。

Observed:

- `cargo test -p dimweave codex_port_pool` -> `17 passed; 0 failed`
- `cargo test -p dimweave codex_port_in_use_errors_are_retryable` -> `1 passed; 0 failed`
- `cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead` -> `1 passed; 0 failed`
- `cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui` -> `1 passed; 0 failed`
- `cargo test -p dimweave agent_id_routing_auto_same_provider` -> `2 passed; 0 failed`
- `cargo test -p dimweave task_provider_summary_same_provider_uses_each_agent_connection` -> `1 passed; 0 failed`
- `cargo test -p dimweave coder_prompt_falls_back_to_user_when_no_lead_available` -> `2 passed; 0 failed`
- `bun test src/stores/bridge-store/listener-setup.test.ts` -> `14 pass; 0 fail`
- `git diff --check` -> clean

- [ ] **Step 2: 手工 dev 验证**

Run:

```bash
cd /Users/jay/floder/dimweave
bun run tauri dev
```

验证：

1. 保留旧 `codex app-server` 占住 `4500` / `4501`，daemon 占 `4502`，新建 coder 应分配到 `4503` 或后续可用端口。
2. 新 task 只有 coder 或 lead 离线时，coder 完成结果应出现在 GUI，不应无限 buffer 到 lead。
3. 多 task 切换时，Codex/Claude stream 指示器只更新对应 task bucket，切回 task 后能恢复该 task 的 stream 状态。

- [x] **Step 3: 回填 CM**

执行每个 commit 后，把本节末尾的 `Commit` 从 `未提交` 改成真实 hash，并把验证列替换为真实输出摘要。

- [x] **Step 4: 提交 CM 文档**

```bash
git add docs/superpowers/plans/2026-04-26-codex-coder-connect-recovery.md
git commit -m "docs: backfill CM for Codex coder connect recovery"
```

## CM 回填区

| Task | Commit | Summary | Verification | Status |
|---|---|---|---|---|
| Task 1 | `78cbd10` | Codex port pool 跳过 daemon 端口和 OS 已占用端口；Codex launch / resume / attach 遇到 `Port N still in use...` 时释放当前 lease 并重试下一个端口一次。 | `cargo test -p dimweave codex_port_pool` -> `17 passed; 0 failed`; `cargo test -p dimweave codex_port_in_use_errors_are_retryable` -> `1 passed; 0 failed`; `git diff --check` -> clean | focused-verified |
| Task 2 | `c20f9b1` | Worker 回复 lead 时改为检查在线 lead；没有在线 lead 时回退到 user；删除 dispatch 外层重复 retarget；同步更新 role prompt。 | `cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead` -> `1 passed; 0 failed`; `cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui` -> `1 passed; 0 failed`; `cargo test -p dimweave coder_prompt_falls_back_to_user_when_no_lead_available` -> `2 passed; 0 failed`; `git diff --check` -> clean | focused-verified |
| Task 3 | `ae7241d` | same-provider lead/coder 改为按 agent_id 判断在线和 connection，避免 provider-level slot 混淆。 | `cargo test -p dimweave agent_id_routing_auto_same_provider` -> `2 passed; 0 failed`; `cargo test -p dimweave task_provider_summary_same_provider_uses_each_agent_connection` -> `1 passed; 0 failed`; `git diff --check` -> clean | focused-verified |
| Task 4 | `df017f5` | pending stream queue 记录 bucket task id，flush 时优先写入 queue-time task bucket，避免 active task 切换窗口写错桶。 | `bun test src/stores/bridge-store/listener-setup.test.ts` -> `14 pass; 0 fail`; `git diff --check` -> clean | focused-verified |
| Task 5 | N/A | CM 文档回填；本 docs commit 不自引用最终 hash。 | 自动化聚焦验证全部通过；手工 UI/E2E 未执行 | documented |

## 当前聚焦验证结果

- `cargo test -p dimweave codex_port_pool` -> `17 passed; 0 failed`
- `cargo test -p dimweave codex_port_in_use_errors_are_retryable` -> `1 passed; 0 failed`
- `cargo test -p dimweave worker_reply_to_missing_lead_surfaces_to_gui_when_task_has_no_lead` -> `1 passed; 0 failed`
- `cargo test -p dimweave worker_reply_to_offline_lead_surfaces_to_gui` -> `1 passed; 0 failed`
- `cargo test -p dimweave agent_id_routing_auto_same_provider` -> `2 passed; 0 failed`
- `cargo test -p dimweave task_provider_summary_same_provider_uses_each_agent_connection` -> `1 passed; 0 failed`
- `cargo test -p dimweave coder_prompt_falls_back_to_user_when_no_lead_available` -> `2 passed; 0 failed`
- `bun test src/stores/bridge-store/listener-setup.test.ts` -> `14 pass; 0 fail`
- `git diff --check` -> clean

## 手工验证备注

- `bun run tauri dev` 会话已在本地保持运行，但本轮没有执行 UI 点击式 E2E。
- 因此本 plan 的完成状态是“代码 task 已提交 + 自动化聚焦验证通过 + CM 已回填”，不是完整手工验收完成。
