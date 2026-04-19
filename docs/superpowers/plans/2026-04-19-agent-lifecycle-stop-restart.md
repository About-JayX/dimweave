# Agent 生命周期修复：删除真停、编辑重启

## 问题（用户反馈）

1. 编辑 agent 保存 `Save & Connect` 后，运行中 agent **不会**按新 model/effort 切换
2. 删除 agent 后，子进程**仍在**响应路由消息

## 根因

两条都在同一层 —— daemon 的分发从来没按 `agent_id` 停过正在跑的子进程。

- **编辑不生效**：`DaemonCmd::LaunchClaudeSdk` 和 `DaemonCmd::LaunchCodex` 开头有短路：
  ```rust
  if rt.claude_slot_by_agent(eid).map_or(false, |sl| sl.is_online()) {
      gui::emit_system_log(... "already online, skipping launch");
      let _ = reply.send(Ok(()));
      continue;
  }
  ```
  同一 `agent_id` 的第二次 launch 被直接吞掉 → 旧子进程带旧 config 继续跑。

- **删除还响应**：`DaemonCmd::RemoveTaskAgent` handler 只做 `s.task_graph.remove_task_agent(&agent_id)` + save，不 kill 子进程。Claude/Codex handle 仍在 `claude_sdk_handles` / `codex_handles` 这两个 HashMap 里，routing 按 agent_id 继续找到它。

## 实现

### 1. 统一 stop 辅助函数

`src-tauri/src/daemon/mod.rs` 新 `async fn stop_agent_by_id(codex_handles, claude_sdk_handles, codex_port_pool, agent_id, state, app)`：
1. 从 `task_graph` 查 `agent_id` 所属 `task_id`（后续 invalidate 需要）
2. `codex_handles.remove(agent_id)` → 存在则 `h.stop().await` + `port_pool.release(...)` + `state.invalidate_codex_agent_session(tid, agent_id)`
3. `claude_sdk_handles.remove(agent_id)` → 存在则 `h.stop().await` + `state.invalidate_claude_agent_session(tid, agent_id)`
4. 发 `emit_task_context_events` 刷新 UI

调 `is_codex_online()` 判断是否需要广播 provider 离线（没有 Claude 对应方法，省略）。

### 2. 新 `DaemonCmd::StopAgent`

- `src-tauri/src/daemon/cmd.rs` 加变体 `StopAgent { agent_id, reply }`
- `src-tauri/src/daemon/mod.rs` handler 直接调 `stop_agent_by_id`

### 3. `RemoveTaskAgent` 内嵌 stop

- 在 `s.task_graph.remove_task_agent(&agent_id)` **之前** 调 `stop_agent_by_id`
- 这样 routing 立刻停止响应 → 再删 task_graph 行 → emit events

### 4. Tauri command + TS 贯通

- `src-tauri/src/commands_task.rs::daemon_stop_agent` 包装 `StopAgent`
- `src-tauri/src/main.rs` 注册
- `src/stores/task-store/types.ts` + `index.ts` 增 `stopAgent(agentId)` action
- `src/components/TaskPanel/index.tsx::handleDialogSubmit`（edit 分支）：
  ```ts
  const savedAgents = await handleEditSubmit(payload);
  if (payload.requestLaunch && task && savedAgents.length > 0) {
    for (const a of savedAgents) {
      if (a.agentId) await stopAgent(a.agentId);  // kill 老进程
    }
    await launchProviders(task.taskId, task.taskWorktreeRoot, savedAgents);
  }
  ```

## 语义

- **Save + Connect**（edit）→ 强制 restart，**哪怕 config 未变**。简单可预期，接受短暂 offline 窗口
- **Save 不 Connect** → 只改 DB，不重启；现有 agent 继续跑旧 config
- **Remove** → 总是 kill 子进程 + 释放端口 + 清 runtime 槽 + 删行
- **Claude / Codex 切换后历史**：launch 时 `resumeSessionId` / `resumeThreadId` 会续上旧 external session，所以对话不丢

## 文件清单

| 文件 | 改动 |
|---|---|
| `src-tauri/src/daemon/mod.rs` | `stop_agent_by_id` helper + RemoveTaskAgent/StopAgent handler |
| `src-tauri/src/daemon/cmd.rs` | `StopAgent` 变体 |
| `src-tauri/src/commands_task.rs` | `daemon_stop_agent` command |
| `src-tauri/src/main.rs` | 注册 command |
| `src/stores/task-store/types.ts` + `index.ts` | `stopAgent` action |
| `src/components/TaskPanel/index.tsx` | edit 流程先 stop 再 launch |

## 不做的事

- 不做 "diff config, 只在需要时 restart" 的优化 —— 比较逻辑复杂，Save & Connect 语义就是"应用新配置"，全量 restart 可以接受
- 不区分 subprocess 级端口 vs 整个 task port_pool —— 现有 release 逻辑按 `task_id + launch_id` 已经够精准
- 不做 Delete 确认（`ConfirmDialog` 只在 "Delete Task" 上）—— 单 agent remove 是编辑流程的一环，不需要额外确认

## 验证

- `cargo test` — 738 passed，无回归
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：
  1. 启动 agent → 编辑改 model → Save & Connect → daemon 日志出现 stop + 新 launch 并带新 model 参数
  2. 启动 agent → 编辑删除该 agent → Save → agent 立刻从 routing 消失（不再响应）

## CM 回填区

- `f3a56f51` — `fix(agents): stop subprocess on remove, restart on edit` — 两个问题一次修复
