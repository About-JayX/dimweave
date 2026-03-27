# AgentBridge 全链路审计总结

> 目的：把本轮及相关连续审计过程收敛成一份总文档，覆盖前端、Tauri daemon、Codex app-server、Claude bridge、MCP 通道与消息路由。
> 说明：本文件是“总摘要”。协议细节和专项修复仍分别保留在 `docs/agents/claude-chain.md`、`docs/agents/codex-chain.md` 等文档中。

## 标记说明

- `[已修复]`：问题已在代码中落地修复，并在后续审计中被复核通过。
- `[未修复]`：问题在审计时确认存在，当前仓库尚未处理。
- `[已知限制]`：不是当前阻断缺陷，但能力仍不完整或可见性不足。
- `【重复问题】`：同一根因或同一问题簇在多轮审计中重复出现，或第一次修复后又暴露出更深一层的同类问题。

## 审计范围

本轮审计覆盖的主要链路：

- 前端：`src/components`、`src/stores`
- Tauri daemon：`src-tauri/src/daemon`
- Codex 会话：`src-tauri/src/daemon/codex`
- Claude bridge / MCP：`bridge/src`
- 启动与命令入口：`src-tauri/src/main.rs`、`src-tauri/src/commands.rs`

## 提交时间线

以下提交对应本轮连续审计后的主要修复节点：

| 提交 | 主题 | 说明 |
|------|------|------|
| `068ec9b` | performance + logic chain audit | 生命周期、背压、渲染链初步修复 |
| `0cdc4cc` | turn-level routing + bridge lossless reconnect + target picker UI | turn 级路由、bridge 重连、目标选择器首次接通 |
| `4ba60e0` | remove `user` from target picker | 收紧目标选择器语义 |
| `7c52ec3` | target routing + auto-mode broadcast + response rules | 目标路由和多 agent 响应规则成型 |
| `5f218d2` | hide unwired reasoning UI | 隐藏未接通的 reasoning 控件 |
| `19acc61` | fix 5 issues | Connect 按钮、多目标输入、重连竞态、auto 去重、`codexReady` |
| `4f95475` | auto online-only broadcast, buffer migration, replay safety, codex role sync | 在线广播、buffer 迁移、daemon replay 回填、Codex role 同步 |
| `b659fea` | role uniqueness, replay tail preservation, codex inject safety | 角色唯一性、replay 尾部保留、Codex inject 安全 |
| `5ce19b2` | permission auto-deny, pre-init buffer safety, codex status cleanup | permission 自动拒绝、pre-init buffer 安全、Codex 状态清理 |

## 问题簇总览

### 1. 【重复问题】消息路由、重复消息与反馈环

这是本轮最早暴露、也最反复的一组问题。

- [已修复] schema-route 最初是 session 级布尔值，无法绑定到具体 turn，导致跨 turn 串台。
- [已修复] 后续改为 `request_id -> turn_id -> from_user` 映射，turn 归属关系才真正闭合。
- [已修复] Codex 侧早期存在“双通道”风险：文本结构化路由和工具回复语义重叠，容易造成语义重复发送。
- [已修复] `target` 选择器一度只是死 UI，修复后前端目标选择才真正进到路由链路。
- [已修复] `auto` 模式一度会对同角色双发，之后通过去重和角色唯一性约束关闭了这条路径。
- [已修复] 角色唯一性最终在 `b659fea` 收口，否则同一 role 被 Claude/Codex 同时占用时，daemon 路由会优先命中 Claude，造成 Codex 不可达。

结论：

- 这组问题已经从“重复消息 / 串台 / 不可达”演进到“单通道、单角色、单次路由”的更稳定模型。

### 2. 【重复问题】缓冲与回放的无损性

这是整个项目里第二个反复出现的问题簇，横跨 bridge、daemon、Codex inject 和 MCP pre-init 缓冲。

- [已修复] bridge 重连早期会在 backoff 或发送失败时直接丢消息。
- [已修复] bridge 之后改为保留 `pending` 和 `remaining`，重连时能回放未送达消息。
- [已修复] daemon 侧最初只在 replay 失败时回填“当前项”，尾部 backlog 仍会丢失。
- [已修复] `b659fea` 后，control replay、permission verdict replay、Codex inject replay 都改成了“失败点及其后续尾部统一回填”。
- [已修复] MCP `pre_init_buffer` 一度无上限，且初始化后 replay 失败会丢掉未处理 tail。
- [已修复] `5ce19b2` 后，`pre_init_buffer` 有上限，初始化 replay 也改成保留未处理项。

结论：

- “消息不丢”不是一次修复完成的，而是沿着 `bridge -> daemon -> codex inject -> MCP pre-init` 四层逐层补齐的。
- 这一问题簇应视为已经被系统性收敛，而不是某一个提交单点解决。

### 3. 【重复问题】角色状态同步与角色唯一性

- [已修复] Codex role 最初只存在前端 store，daemon 不知道当前选择，状态快照和重载后会回退。
- [已修复] `4f95475` 增加 `daemon_set_codex_role` 后，Codex role 才具备后端事实来源。
- [已修复] 角色切换后，buffer 曾经会遗留在旧 role 上，形成孤儿消息。
- [已修复] `migrate_buffered_role` 接入后，旧 role 上的缓冲消息会重定向到新 role。
- [已修复] 但仅有迁移还不够，因为两个 agent 仍可短暂共用同一 role，导致迁移和路由都出现歧义。
- [已修复] `b659fea` 最终在 UI 层过滤掉“另一个 agent 已占用的 role”，把角色唯一性前置约束起来。
- [已知限制] 最新复核发现，前端 `setRole` 仍是乐观写入，daemon 对冲突 role 的拒绝结果没有回传到前端；因此“daemon 是 role 的事实来源”在 UI 边界上还没有完全闭环。

结论：

- 角色同步问题最终收敛成两个前提：
- daemon 是 role 的事实来源。
- 同一时刻一个 role 只能由一个 agent 占用。

### 4. UI 表示与真实状态不一致

- [已修复] `Connect Codex` 一度因为 `codexReady` 绑定方式错误而永久禁用。
- [已修复] `Reasoning` 控件曾经未接线却暴露给用户，现已隐藏。
- [已修复] Codex 头部一度用假的 `ready/connecting` 状态掩盖真实连接状态，`5ce19b2` 已移除这套占位状态。
- [已知限制] `threadId` 仍未从 daemon 透传到前端，所以头部虽然支持显示 thread，但实际仍传 `null`。

结论：

- 当前 UI 层最大的剩余问题不是错误路由，而是可见性仍不完整，尤其是 thread id。

### 5. 安全性与权限链

- [已修复] bridge 入站消息曾经可伪造 sender，现已绑定到认证后的 agent role。
- [已修复] GUI 曾经在真正路由前先展示消息，导致 ghost message；后续改成“先路由后展示”。
- [已修复] permission verdict 在 agent 离线时已支持 daemon 侧缓冲。
- [已修复] Claude `permission_request` 后来又暴露出另一层问题：当发往 daemon 的内部通道关闭时，请求会静默吞掉，Claude 端永远等待。
- [已修复] `5ce19b2` 将此路径改为自动 `deny`，避免 Claude 无限挂起。

结论：

- 权限链当前更偏“保守失败”，即通道异常时优先显式拒绝，而不是静默悬挂。

## 逐轮问题记录

### 第一阶段：路由与会话归属

- [已修复] 多条 message 的根因不是单点 bug，而是路由出口不收敛、turn 归属建模不完整、UI 提前展示三者叠加。
- [已修复] `turn/start` 与 `turn/started` 的对应关系最终改成按 request/turn 建模，而不是 session 级开关。
- [已修复] sender spoofing、ghost message 这两类问题都已经在 daemon 路由层关闭。

### 第二阶段：目标路由与前端交互

- [已修复] `target` 选择器接线。
- [已修复] `auto` 模式去重。
- [已修复] 输入框不再只依赖 Codex 在线；现在至少能面向任一在线 agent 发送。
- [已修复] Connect/Disconnect 的基本交互链已经稳定。

### 第三阶段：重连、buffer 与 replay

- [已修复] bridge reconnect 改成尽量无损。
- [已修复] daemon replay 和 Codex inject replay 起初仍有尾丢失，后续补成整段尾回填。
- [已修复] MCP pre-init buffer 也补上了容量限制与 replay 尾保留。

### 第四阶段：角色一致性

- [已修复] `daemon_set_codex_role` 接通后，Codex role 不再只是前端状态。
- [已修复] role change 时的 buffer orphan 已通过迁移修复。
- [已修复] 最终通过角色唯一性约束，把同 role 双占用从源头禁止。

### 第五阶段：权限与状态可见性

- [已修复] permission request 静默吞没已改为 auto-deny。
- [已修复] CodexHeader 的假 `ready` 状态已清理。
- [已知限制] `threadId` 仍未送到前端，状态可见性尚未闭环。

## 当前状态

按本轮连续审计结束时的代码状态，核心链路可归纳为：

- 消息路由：已基本稳定
- reconnect / replay：已基本稳定
- 角色同步：已基本稳定
- 权限链：已收敛到保守失败
- UI 状态展示：主链路可用，但 thread 可见性仍不完整

## 最新补充（基于 `5ce19b2` 后复核）

这一轮补充复核确认，最近三项修复已经稳定落地：

- [已修复] `permission_request` 在发往 daemon 的内部通道关闭时，已改为自动 `deny` 回 Claude，避免审批链路无限挂起。
- [已修复] MCP `pre_init_buffer` 已增加容量上限，并在初始化 replay 失败时保留未处理 tail，而不是继续丢失尾部消息。
- [已修复] Codex 头部的占位 `ready` 语义已删除，状态收敛为真实的 `connected/disconnected`。

同时，最新复核后仍需保留以下残余问题：

### 1. [已修复] role 唯一性现在在 daemon 层有冲突校验

- `set_role` 增加冲突校验：写入前检查另一个 agent 的当前 role，重复则拒绝（返回 false）。
- [已知限制] 前端 `RoleSelect` 只做 UI 过滤，`daemon_set_claude_role` / `daemon_set_codex_role` 当前没有把 reject 结果回传给前端；store 仍先本地写入，UI 与 daemon 状态仍可能漂移。

### 2. [已修复] permission auto-deny 写失败时退出 MCP loop

- `write_line(writer, &deny)` 返回值现在被检查，失败时 `return false` 退出 MCP 循环。
- 退出后 bridge 进程整体终止，Claude 侧感知到 MCP server 断开。

### 3. 【重复问题】最近几轮主修复点的回归测试仍然不足

- [已知限制] 近期连续修复的核心问题大多来自真实链路复查，而不是由自动化测试先发现。
- [已知限制] 当前仓库仍缺少针对以下问题簇的专门回归测试：
  - role 唯一性约束
  - `SetCodexRole` / `SetClaudeRole` 的后端冲突处理
  - `permission auto-deny`
  - `pre_init_buffer` 上限与 replay 保尾

结论：

- 当前代码主链已明显稳定，但自动化验证层仍落后于这几轮修复速度。

### 4. [已修复] user 输入在 auto 模式下显示两条 message 气泡

- [已修复] 根因：前端在 `auto` 模式下会把一次用户输入拆成多条 transport 级 `BridgeMessage`，daemon 又会把每条都 `emit_agent_message` 到 GUI，导致消息面板出现多条一模一样的 user 气泡。
- [已修复] 修复方案：新增 `daemon_send_user_input` Tauri command，前端只发一次。daemon 内部 `route_user_input` 先发一次 GUI echo，再通过 `route_message_silent` 把 transport 副本 fan-out 到各 target，不重复 emit。
- [已修复] 新增 `resolve_user_targets` 纯函数，”auto” 解析为在线 agent roles（去重），fan-out 决策从前端下沉到 daemon。
- [已修复] 7 项 `resolve_user_targets` 回归测试覆盖：显式 target、auto 空/单/双 agent、去重。

### 5. [已修复] 2026-03-27 复核发现的剩余链路问题

- [已修复] `RoleSelect` 已移除 `user` 选项，agent 不再能选择 `user` role。
- [已修复] `resolve_user_targets` 在 auto 模式下过滤掉 `user` role，避免路由黑洞。
- [已修复] `route_user_input` 在零目标时 emit 系统日志警告，用户可观测。
- [已修复] 补充回归测试：`auto_excludes_user_role`、`migrate_buffered_role_retargets_messages`、`take_buffered_for_drains_only_matching_role`、`buffered_verdicts_round_trip`、`buffered_verdicts_cap_at_50`。

## 当前仍需保留的已知限制

- [已知限制] `threadId` 尚未从 daemon 暴露到前端，Codex 头部无法显示真实 thread。
- [已知限制] 部分回归测试仍依赖手动验证，尤其是：
  - daemon replay tail 保留
  - Codex inject replay tail 保留
  - MCP pre-init buffer 安全
  - permission auto-deny + write failure exit
- [已知限制] `RoleSelect` 与 daemon role 拒绝结果之间仍缺少显式回传，role 唯一性在 UI 边界上没有完全闭环。

## 验证记录

本轮审计过程里反复使用的验证命令包括：

```bash
cargo test
npm run build
cargo clippy --workspace --all-targets -- -D warnings
```

在最近一次审计收口时，结论为：

- `cargo test`：通过
- `npm run build`：通过
- `cargo clippy --workspace --all-targets -- -D warnings`：未通过，但失败项仍以 lint/样式问题为主

在 2026-03-27 当前工作区复核时：

- `cargo test`：通过（49 tests）
- `bun run build`：通过（修复 `react-virtuoso` 依赖安装后）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过（修复 12 项 lint 问题后）

## 相关文档

- `docs/agents/claude-chain.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-channel-api.md`
- `docs/agents/codex-app-server-api.md`
- `docs/agents/codex-app-server-api.zh-CN.md`
