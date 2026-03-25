# Per-Agent Role Assignment

> Historical note: this dated spec was written for the Bun-daemon migration period. Current source of truth is `CLAUDE.md`, `.claude/rules/**`, `src-tauri/src/daemon/**`, and `bridge/**`.

## Summary

将统一 `role` 字段拆分为 `claudeRole` + `codexRole`，支持异构角色分配（如 Claude=Lead, Codex=Coder）。角色定义（`ROLES` 常量）保持统一共享。同时修复 review 中发现的 critical/important 问题。

## Motivation

当前统一 `role` 强制 Claude 和 Codex 使用相同角色，无法支持 CLAUDE.md 中定义的角色执行模式（Lead 分任务 → Coder 实现 → Reviewer 审查 → Tester 测试）。按「参数不同就分离，相同就统一」原则：角色选择是 agent 特定的（可不同），角色定义是共享的（相同）。

## Design

### Data Model

**daemon-state.ts:**
```typescript
// Before
role: RoleId = "lead";

// After
claudeRole: RoleId = "lead";
codexRole: RoleId = "coder";
```

**bridge-store types.ts:**
```typescript
// Before
role: string;
setRole: (role: string) => void;

// After
claudeRole: string;
codexRole: string;
setRole: (agent: "claude" | "codex", role: string) => void;
```

### Protocol

**GUI → Daemon (set_role):**
```json
{ "type": "set_role", "agent": "claude", "role": "lead" }
{ "type": "set_role", "agent": "codex", "role": "coder" }
```

**Daemon → GUI (role_sync):**
```json
{
  "type": "role_sync",
  "payload": { "claudeRole": "lead", "codexRole": "coder" }
}
```

### Daemon Logic

**role-actions.ts — `handleSetRole`:**
- 接收 `{ agent, role }` 参数
- 根据 `agent` 更新 `state.claudeRole` 或 `state.codexRole`
- 如果改的是 codexRole 且 Codex 有活跃 session → 重连 Codex（用新角色的 sandbox/instructions）
- 如果改的是 claudeRole → 只广播（Claude PTY 需要重启才能生效，不自动重启）
- 广播 `role_sync` 事件（包含两个角色）
- 失败回滚时发送完整的 `{ claudeRole, codexRole }` 对象，只回滚失败 agent 的角色

**codex-actions.ts — `handleLaunchCodexTui` / `handleApplyConfig`:**
- `ROLES[daemonState.role]` → `ROLES[daemonState.codexRole]`
- 两处引用都改为 codexRole（line 46 和 line 128）

**codex-events.ts — 转发逻辑:**
- `state.role` → `state.codexRole`（3 处：line 87 角色查找, line 107/108 日志消息）

**daemon.ts — `currentStatus()`:**
- 新增 `claudeRole` 和 `codexRole` 字段到状态快照

**gui-server/server.ts — WebSocket `open`:**
- 连接后额外发送 `role_sync` 事件，确保新客户端获取当前角色

### Frontend

**AgentStatus/index.tsx:**
- 移除顶部 `RoleSelect`

**ClaudePanel/index.tsx:**
- 新增 `RoleSelect`，绑定 `claudeRole`
- `useBridgeStore((s) => s.role)` → `useBridgeStore((s) => s.claudeRole)`
- `buildClaudeAgentsJson(role)` → `buildClaudeAgentsJson(claudeRole)`
- `roleId: role` → `roleId: claudeRole`
- deps array 中的 `role` → `claudeRole`
- 锁定状态（connected/running）时 disabled

**CodexPanel.tsx:**
- 新增 `RoleSelect`，绑定 `codexRole`
- Codex 运行时 disabled（改角色需要重连）

**RoleSelect.tsx:**
```typescript
// Before
export function RoleSelect({ disabled }: { disabled?: boolean })

// After
export function RoleSelect({ agent, disabled }: {
  agent: "claude" | "codex";
  disabled?: boolean;
})
```
从 store 读取对应 agent 的角色，调用 `setRole(agent, role)`。

**message-handler.ts:**
```typescript
case "role_sync": {
  const { claudeRole, codexRole } = guiEvent.payload;
  set({ claudeRole, codexRole });
  break;
}
```

**types.ts (DaemonStatus):**
- 新增 `claudeRole` 和 `codexRole` 可选字段

### Edge Cases

**快速连续改角色的竞争条件：**
角色切换时如果 Codex 正在重连中，新的 `set_role` 应该直接覆盖 `state.codexRole`。旧重连的 `.then()` 回调检查当前 `state.codexRole` 是否和自己发起时一致，不一致则跳过广播（已被更新的请求取代）。

**实现方式：** 在 `handleSetRole` 中记录一个递增的 `roleChangeGeneration` 计数器。重连回调持有发起时的 generation，完成时比对，不匹配则静默跳过。

### Bugfixes (bundled)

1. **stop_pty 死锁** — `child.wait()` 在持锁时阻塞。改为先 `take()` child，释放锁，再 `wait()`。
2. **空 forwardPrompt** — `user`/`lead` 角色 `forwardPrompt` 为空。在 `codex-events.ts` 注入时 fallback：空 forwardPrompt 时使用 `${codexRole.label} says:` 前缀（当前已有此逻辑）。额外给 `user` 角色加一个有意义的 `forwardPrompt`。
3. **死代码** — 删除未使用的 `ClaudeRoleSelect.tsx`。
4. **CLI 参数校验** — `launch_pty` 中验证 model/effort 白名单。

## Files Changed

| File | Change |
|------|--------|
| `daemon/daemon-state.ts` | `role` → `claudeRole` + `codexRole` |
| `daemon/daemon.ts` | `currentStatus()` 加 `claudeRole`/`codexRole` 字段 |
| `daemon/gui-server/role-actions.ts` | `handleSetRole` 接收 `{ agent, role }`，加 generation 防竞争 |
| `daemon/gui-server/codex-actions.ts` | `daemonState.role` → `daemonState.codexRole` (2 处) |
| `daemon/gui-server/server.ts` | WebSocket open 时发送 `role_sync` |
| `daemon/codex-events.ts` | `state.role` → `state.codexRole` (3 处) |
| `daemon/role-config/roles.ts` | `user` 角色加有意义的 `forwardPrompt` |
| `src/types.ts` | `DaemonStatus` 加 `claudeRole`/`codexRole` 可选字段 |
| `src/stores/bridge-store/types.ts` | `role` → `claudeRole` + `codexRole` |
| `src/stores/bridge-store/index.ts` | `setRole(agent, role)`, 初始值改为两个角色 |
| `src/stores/bridge-store/message-handler.ts` | `role_sync` 解构两个角色 |
| `src/components/AgentStatus/RoleSelect.tsx` | 加 `agent` prop |
| `src/components/AgentStatus/index.tsx` | 移除顶部 RoleSelect |
| `src/components/ClaudePanel/index.tsx` | 加 RoleSelect + `role` → `claudeRole` (4 处) |
| `src/components/AgentStatus/CodexPanel.tsx` | 加 RoleSelect |
| `src/components/ClaudePanel/ClaudeRoleSelect.tsx` | 删除（死代码） |
| `src-tauri/src/pty.rs` | 修复 stop_pty 死锁 + model/effort 校验 |

## Not Changed

- `ROLES` 定义结构不变（角色定义是共享的）
- `RoleId` 类型不变
- `ROLE_OPTIONS` 不变
- `agent-roles.ts` 不变（前端 Claude 专用的 agents JSON 生成器）
- 编排器 / session-manager 不变（v1 scope）
- `daemon/gui-server/handlers.ts` 不变（已转发到 role-actions）
