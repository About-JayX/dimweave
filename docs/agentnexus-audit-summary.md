# AgentNexus 全链路审计总结

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
- [已修复] `Reasoning` 控件最初未接线，后续被直接隐藏；当前已重新接线并恢复为真实可用配置。
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

### 6. [已修复] 2026-03-27 深度复核（基于 `0a9b833f`）发现的残余问题

- [已修复] daemon API 边界 role 白名单：`AGENT_ROLES` 白名单 + `is_valid_agent_role()` 校验已加入 `set_role`、`daemon_set_claude_role`、`daemon_set_codex_role`、`daemon_launch_codex`、`daemon_send_user_input`。`”user”` 和非法 role 在 Tauri command 层即被拒绝。
- [已修复] zero-target 语义：`route_user_input` 现在在 `targets.is_empty()` 时 **不再** emit GUI echo，只写 warn 日志并直接 return，避免”看似已发送”的假气泡。
- [已修复] 行为级回归测试：新增 `auto_fanout_delivers_to_both_agents`、`explicit_user_target_routes_to_gui`、`valid_roles_accepted`、`user_role_rejected`、`unknown_role_rejected`（共 5 项），覆盖 fan-out 投递、user target 路由、role 白名单。

### 7. [已修复] `daemon_send_message` 旁路 API 移除

- [已修复] `daemon_send_message` 已从 `invoke_handler` 移除，Tauri command handler 和 `DaemonCmd::SendMessage` 变体均已删除。前端不再能绕过 role 白名单和 `route_user_input` 语义。
- 内部 daemon 代码（bridge `AgentReply`、Codex structured output routing）直接调用 `routing::route_message`，不经过 Tauri command 层，不受此变更影响。

### 8. [已修复] 内部 agent 路由对非法 target 的校验

- [已修复] Claude bridge `reply` tool schema 的 `to` 字段已加 `enum` 约束（`[“user”,”lead”,”coder”,”reviewer”,”tester”]`），`handle_tool_call()` 拒绝非法 target 返回 `None`。
- [已修复] `routing.rs` 的 `route_message_inner` 对不匹配当前 Claude/Codex role 且不在 `AGENT_ROLES` 白名单中的 target，改为 `RouteResult::Dropped` 而不是 `NeedBuffer`，不再污染 `buffered_messages`。
- [已修复] 新增测试：`invalid_target_rejected`（bridge）、`reply_schema_has_enum_constraint`（bridge）、`invalid_target_is_dropped_not_buffered`（daemon）、`valid_role_offline_is_buffered`（daemon）。

### 9. [已修复] 2026-03-27 交互链路修复（Messages / Claude Terminal / Thinking）

- [已修复] Codex streaming 现在只显示结构化输出中的 `message`，不再把 `{"message":"...","send_to":"..."}` 原样泄漏到消息区。daemon 在 `item/agentMessage/delta` 阶段维护原始缓冲，并提取可展示 preview；前端 `codex_stream.delta` 语义同步收紧为“当前可展示 message 预览”，不再做字符串拼接。
- [已修复] Claude 新增 `claude_stream` 事件链（`thinkingStarted` / `preview` / `done` / `reset`）。Messages 面板会在消息成功投递给 Claude 后显示稳定的 Claude thinking 占位；当前前端已不再消费 preview 文本，避免 PTY 摘要噪音直接进入消息区。Claude 回 reply、终端退出、显式断开或手动 stop 时会清空 thinking。
- [已修复] Claude terminal attention 现在不仅会自动切到 `Claude Terminal` tab，还会通过前端 `claudeFocusNonce` 强制把键盘焦点放进 xterm，不再要求用户再点一次。该聚焦不依赖 `connected === true`，启动期 prompt 也能抢到输入焦点。
- [已修复] 切回 `Messages` tab 时，`react-virtuoso` 会在组件重新挂载后立即跳到 `LAST`，不再回到顶部；tab 内正常 followOutput 和手动“Back to bottom”行为保持不变。
- [已修复] 空消息过滤已收口到多层：
  - bridge `reply` tool 拒绝空白 `text`
  - Codex 完成消息若 `message.trim().is_empty()`，不再 emit 最终 message，也不再路由
  - daemon `routing.rs` 在 GUI 展示前会过滤空 `BridgeMessage.content`
  - 前端 `MessagePanel` 最终也会过滤空白消息，避免空气泡

当前这轮修复后，主链路的剩余风险已从“交互语义错误”下降为“前端自动化测试覆盖不足”；功能行为以本节与专项链路文档为准。

### 10. [已修复] 2026-03-27 基于 superpowers reviewer 的深度复核收口

- [已修复] `src-tauri/src/daemon/codex/session.rs` 与 `src/stores/bridge-store/helpers.ts` 已拆分回 200 行以内：Codex 结构化输出预览解析被提取到 `src-tauri/src/daemon/codex/structured_output.rs`，前端 listener 逻辑被提取到 `src/stores/bridge-store/listener-setup.ts` 与 `listener-payloads.ts`，单文件复杂度明显下降。
- [已修复] `item/completed(agentMessage)` 中重复的 `should_emit_final_message(&display_text)` 守卫已收敛为单次 early return；空消息不会再先跑一段 `valid_target` 推导，再进入后续 GUI emit / route 分支。
- [已修复] `claude_terminal_attention` 在用户已经停留在 `Claude Terminal` tab 时，前端现在会显式清空 store 中的 `claudeNeedsAttention`，不再留下残余 attention 状态把后续 tab 切换强行弹回 Claude。
- [已修复] `ClaudeTerminalPane` 的 focus effect 已缩减为只由 `focusNonce` 驱动；`connected` / `running` 状态变化不再额外触发 `terminal.focus()`，抢焦点副作用已移除。
- [已修复] `MessageList` 的冗余 `active` prop 已删除，组件语义恢复为“挂载即跳到底部”；tab 可见性边界继续由 `MessagePanel` 的条件渲染承担。
- [已修复] reviewer 顺带指出的规则漂移也已同步：`.claude/rules/frontend.md` 与 `.claude/rules/tauri.md` 已补入 `claude_stream` 事件，避免后续链路审查继续基于过时协议。

### 11. [已修复] 2026-03-27 基于 superpowers reviewer 第二轮复核收口

- [已修复] `src-tauri/src/daemon/routing.rs` 已继续拆分到 200 行以内：展示副作用、Claude thinking 判定和 route log 输出被提取到 `src-tauri/src/daemon/routing_display.rs`，主路由文件重新聚焦在 target 解析与投递。
- [已修复] `MessageList` 初始滚底逻辑不再依赖“组件首次挂载时就已有消息”。当前实现改为基于 `totalCount` 的一次性 auto-scroll：列表若先以空态挂载，后续第一次收到消息时也会跳到底部。
- [已修复] `MessagePanel` 不再直接调用 `useBridgeStore.setState(...)` 清 `claudeNeedsAttention`；前端 store 新增 `clearClaudeAttention()` action，attention 清理路径与其它状态更新方式保持一致。
- [已修复] `codex_stream.delta` 在前端 store 中继续采用“覆盖当前 preview”而不是“追加 token”的消费语义，并在 listener 侧补了注释，明确这是 daemon 端结构化 preview 协议的一部分，不是误删了历史累积逻辑。

### 12. [已修复] 2026-03-27 基于 superpowers reviewer 第三轮复核收口

- [已修复] Claude thinking 启动判定不再通过 `route_message_with_display()` 与 `route_message_inner()` 两次分离读 `claude_role` 来完成。当前路由层新增 `RouteOutcome`，把“是否成功投递”和“是否应该启动 Claude thinking”绑定在同一份 daemon state 快照里，消除了角色切换瞬间的竞态窗口。
- [已修复] `src-tauri/src/daemon/routing.rs` 再次按职责拆分：用户输入 fan-out 与 `auto` 目标解析被提取到 `src-tauri/src/daemon/routing_user_input.rs`，核心路由文件回到 156 行。
- [已修复] `codex_stream.delta` 的前端消费路径重新补回长度上限，当前 preview state 会截断到最近 100,000 个字符，避免长回复重新引入 UI 内存膨胀回归。

### 13. [已知问题] 2026-03-27 项目级深度复核

- [已修复] Codex 结构化输出 preview 现在已在 daemon 侧加入 `RAW_DELTA_CAP = 512_000` 上限，不再无限累积原始 `raw_delta`。本节保留仅用于说明当时复核结论已在后续轮次收口。
- [已知问题] `RoleSelect` / store 的 optimistic role 更新仍未真正闭环。前端 `setRole()` 先直接写 `claudeRole` / `codexRole`，而 Tauri `daemon_set_*_role` command 只返回“命令是否成功入队”；daemon 内部若因为重复 role 拒绝变更，只会写一条 system log，不会把拒绝结果回传给前端，因此 UI role 仍可能和实际 daemon 路由状态分叉。

### 14. [已修复] 2026-03-27 现场故障修复（Codex 4500 端口 / Claude 终端空白）

- [已修复] Codex 启动前不再只是被动等待 4500 端口释放。当前 `codex::start()` 先走 `ensure_port_available()`，如果发现端口仍被旧 app-server 占用，会主动调用 `kill_port_holder()` 清理孤儿持有者后再重试；只有清理后仍未释放才报 `Port 4500 still in use ...`。新增回归测试覆盖“cleanup 后成功启动”和“cleanup 无效时超时失败”两条路径。
- [已修复] Claude 终端在“channel 已连接 / PTY 正在启动，但还没有任何 terminal chunk”时，不再显示一块没有内容的黑面板。当前 `ClaudeTerminalPane` 会给出明确占位文案：
  - `Claude terminal is starting. Waiting for output…`
  - `Claude is connected. Waiting for terminal output…`
  从而区分“还没输出”和“真正卡死/空白”。

### 15. [已知问题] 2026-03-27 Claude Terminal 历史回溯定位

- [已知问题] `Claude Terminal` “不会自动 force 焦点 / 之后的交互 prompt 不再抢焦点”这条回归，历史上是两次改动叠加出来的：
  - `5480b0f3`（`fix: attention event debounce — fire once, not per PTY chunk`）在 `src-tauri/src/claude_session/prompt.rs` 引入了会话级 `attention_fired` 一次性门闩。它解决了 attention storm，但也意味着同一个 PTY session 内后续 prompt 不会再次发 `claude_terminal_attention`。
  - `90fa8994`（`fix: interaction chain — streaming display, empty filter, routing split`）把 `ClaudeTerminalPane` 的 focus effect 从 `connected` 驱动改成只由 `focusNonce` 驱动。结果是：终端是否自动拿到键盘焦点，完全依赖新的 attention 事件；一旦上游因为 `attention_fired` 不再发事件，键盘上下操作也就跟着失效，除非手动点击终端。
- [说明] `404396bd`（`fix: terminal rendering — remove WebGL, fix scroll, cursor, PTY size, attention`）不是这次“不会 force/不能上下操作”的责任提交。它改的是 WebGL renderer、viewport overflow 和终端主题，方向上是在修复滚动/渲染问题，不是引入当前这条焦点回归。
- [状态] 上述回归现已修复：`prompt.rs` 已从会话级一次性门闩改成 prompt 可见性的边沿触发。

## 当前仍需保留的已知限制

- [已知限制] `threadId` 尚未从 daemon 暴露到前端，Codex 头部无法显示真实 thread。
- [已知限制] 部分回归测试仍依赖手动验证，尤其是：
  - daemon replay tail 保留
  - Codex inject replay tail 保留
  - MCP pre-init buffer 安全
  - permission auto-deny + write failure exit
- [已知限制] `RoleSelect` 与 daemon role 拒绝结果之间仍缺少显式回传（daemon 返回 `Err` 但前端 optimistic 更新 store，需要回滚逻辑）。

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

在 2026-03-27 当前工作区最新复核时：

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（59 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（13 tests）
- `bun test tests/message-panel-view-model.test.ts`：通过（6 tests）
- `bun run build`：通过（仅剩 Vite chunk-size warning，不影响构建成功）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过

在 2026-03-27 对 `0a9b833f` 的深度审查时再次复核：

- `cargo test`：通过
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过
- 结论：当时确认的 role 白名单、zero-target 假气泡、行为级测试缺口，已在后续 `bb3d1044` 中修复；当前剩余问题以下方最新复核结论为准。

在 2026-03-27 对 `bb3d1044` 的深度审查时再次复核：

- `cargo test`：通过（54 tests）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过
- 结论：本轮修复已补上 daemon role 白名单、zero-target guard 和行为级测试。后续 `daemon_send_message` 旁路已在下一提交中移除。

在 2026-03-27 对 `c9c6bb83` 的项目级深度审查时再次复核：

- `cargo test`：通过（54 tests）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过
- 结论：公开 Tauri 旁路 `daemon_send_message` 已移除；当前新增发现是内部 agent 路由对非法 target 仍会走离线缓冲，其中 Claude `reply` tool 缺少目标枚举校验是最直接的入口。

在 2026-03-27 本轮交互修复完成后再次复核：

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（58 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（13 tests）
- `bun test tests/message-panel-view-model.test.ts`：通过（3 tests）
- `bun run build`：通过
- `cargo clippy --workspace --all-targets -- -D warnings`：通过

### 14. [已修复] 2026-03-27 reviewer 首轮严格审计修复

- [已修复] **撤销前端 30s auto-clear timeout**：`ClaudeStreamIndicator.tsx` 中的 `THINKING_TIMEOUT_MS` 定时器已完全移除。该定时器会在 30 秒后把 `thinking` 强制设为 `false`，导致后续真实 `preview` 事件被 `listener-setup.ts` 的 `thinking` 守卫丢弃。daemon 已有完整的完成信号链路（`Done` 在 Claude reply 时由 `control/handler.rs` 发出，`Reset` 在 Claude 断开/终端退出时发出），前端不需要猜测超时。
- [已修复] **bridge CHANNEL_INSTRUCTIONS 与 claude_prompt.rs 一致性**：`bridge/src/mcp_protocol.rs` 的 `CHANNEL_INSTRUCTIONS` 已与 `claude_prompt.rs` 的严格静默规则同步。移除了"Proactively report progress"等宽松表述，新增"Stay completely silent" / "Do NOT call reply()" / "This is absolute" 等约束，确保 Claude channel instructions 与 system prompt 不产生指令冲突。新增回归测试 `initialize_result_includes_silence_rules` 验证这些约束存在且不含宽松指令。
- [已修复] **text_utils.rs 测试覆盖**：`src-tauri/src/claude_session/text_utils.rs` 新增 10 项单元测试，覆盖 CSI 序列清洗、OSC + BEL 终止、OSC + ST 终止、独立 BEL 清理、控制字符过滤、换行/制表符保留、混合场景、`tail_chars`、`normalize_prompt_text`、`extract_terminal_preview` 的 chrome 跳过。
- [已修复] 清理无关未跟踪文件 `hello.ts`。

### 15. [已修复] 2026-03-27 reviewer 二轮深度链路审计修复

- [已修复] **P0: 前端 30s thinking timeout 已撤销** — `ClaudeStreamIndicator.tsx` 中的前端超时自动清理逻辑已经移除，Claude thinking 不再由前端猜测结束。后续又试过 daemon 侧 15 秒 idle timeout，但现场验证会在“Claude 仍在处理但暂时没有终端输出”时提前清空 thinking，因此该 daemon timeout 也已在本轮 #19 撤销，当前只保留真实 `Done/Reset` 事件作为结束信号。
- [已修复] **P1: RoleSelect 与 daemon 真值分叉** — `DaemonCmd::SetClaudeRole` 和 `SetCodexRole` 改为携带 `oneshot::Sender<Result<(), String>>` reply channel。`commands.rs` 的 `daemon_set_*_role` 现在 await daemon 真实校验结果并回传前端。前端 `setRole()` 改为 optimistic + rollback：先写 store 保持 UI 响应，invoke 失败（冲突/非法 role）时立即回滚到 prev 值并通过 `logError` 展示错误。
- [已修复] **P1: Codex raw_delta 无上限内存增长** — `src-tauri/src/daemon/codex/structured_output.rs` 的 `ingest_delta()` 新增 `RAW_DELTA_CAP = 512_000` 字节上限。超出时从 buffer 前端按 char boundary 裁剪，保留最近 512KB。与前端 100K 字符 preview 截断形成双层保护。
- [已修复] **P2: 前端 ANSI regex CSI final byte 覆盖不完整** — `MessageMarkdown.tsx` 和 `ClaudeStreamIndicator.tsx` 的 CSI 正则从 `[A-Za-z]` 修正为 `[@-~]`（覆盖完整 0x40-0x7E final byte range）。两处去重提取为共享 `src/lib/strip-escapes.ts`，与 Rust `text_utils.rs` 语义对齐。新增 `tests/strip-escapes.test.ts` 8 项测试，覆盖 bracketed paste (`ESC[200~`) 等此前遗漏的序列。
- [已修复] **P3: Claude terminal attention 单次触发** — `claude_session/prompt.rs` 不再使用会话级 `attention_fired` 一次性门闩，而是改成“prompt 可见性边沿触发”。同一个 prompt 持续可见时不会重复 emit，prompt 消失后再次出现时会重新 emit `claude_terminal_attention`。这条修复直接恢复了后续交互 prompt 的自动 force focus，终端方向键输入也随之恢复。

### 16. [已修复] 2026-03-27 reviewer 三轮修复补充

- [已修复] **setRole 并发竞态** — 前端 `setRole` 从 optimistic + rollback 改为 **非 optimistic**：store 只在 `invoke` 成功后才更新 role 值。daemon 回复通过 oneshot channel 几乎零延迟，UI 感知不到等待。彻底消除了多次快速切换时 stale rollback 覆盖正确值的竞态。
- [已修复] **RAW_DELTA_CAP 截断后 preview 泄漏 JSON wrapper** — `StreamPreviewState` 新增 `truncated: bool` 标志。一旦 `raw_delta` 因超 512KB 被截断，`ingest_delta()` 直接返回 `None`，保持 `last_preview` 不变（最后一个有效 preview）。`reset()` 清除该标志。新增 3 项回归测试：`raw_delta_cap_enforced`、`truncation_does_not_leak_json_wrapper`（验证 `send_to` 不泄漏）、`truncated_flag_resets_on_new_turn`。

## 验证记录（本轮 #16）

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（72 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（14 tests）
- `bun test tests/`：通过（15 tests across 3 files）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过
- `bun x tsc --noEmit -p tsconfig.app.json`：通过

### 17. [已修复] 2026-03-27 现场复核补充（Claude force focus / PTY watcher panic）

- [已修复] **主窗口前置缺口** — 上一轮修复只做到了前端 `ClaudeTerminalPane.terminal.focus()`，但没有在 attention 到来时把 Tauri 主窗口本身拉回前台。现场如果 App 已失焦或被最小化，方向键仍不会进入 xterm，看起来就像“不会自己 force”。当前 daemon 在 `emit_claude_terminal_attention()` 前会先对主窗口执行 `show -> unminimize -> set_focus`，然后前端再用 `claudeFocusNonce` 聚焦 xterm。
- [已修复] **`claude-pty-watch` 运行态 panic** — 现场日志显示 `thread 'claude-pty-watch' panicked ... there is no reactor running`。根因是 PTY watcher 跑在普通 `std::thread`，但 `gui.rs` 的 Claude thinking idle timeout 用了 `tokio::spawn`。当 watcher 线程里发出 `claude_stream.preview` 时，会直接在无 Tokio reactor 的线程上 panic，导致 preview / attention / auto-confirm 后续全部失效。当前已改为 `tauri::async_runtime::spawn`，不再依赖调用线程自带 Tokio runtime。

### 18. [已修复] 2026-03-27 Claude thinking UI 做减法

- [已修复] **Claude preview 不再进入前端状态/UI** — 由于 PTY 摘要无法稳定捕捉，当前前端改为只保留单一 Claude `thinking…` 占位。`listener-setup.ts` 里的 `handleClaudeStreamEvent()` 现在直接忽略 `preview` payload，不再把文本写进 `claudeStream.previewText`。
- [已修复] **Claude indicator 只依赖 `thinking`** — `MessageList` / `view-model` 不再把 `previewText` 当成 Claude indicator 的显示条件；`ClaudeStreamIndicator.tsx` 也不再渲染 preview 内容。即使 daemon 仍发 `claude_stream.preview`，消息区只会看到一个稳定的 Claude thinking 卡片。

## 验证记录（本轮 #18）

- `bun test tests/message-panel-view-model.test.ts`：通过（11 tests）
- `bun test tests/claude-stream-reduction.test.ts`：通过（1 test）
- `bun test tests/`：通过（20 tests across 4 files）
- `bun run build`：通过

### 19. [已修复] 2026-03-27 Claude thinking 不再被静默超时提前结束

- [已修复] **daemon 15 秒 idle timeout 已移除** — 用户现场复现了新的真实问题：Claude 终端一段时间没有输出，但 reply 实际还没结束，Messages 面板里的 Claude thinking 已经消失。根因是 `src-tauri/src/daemon/gui.rs` 里的 idle timeout 会在静默 15 秒后主动 emit `claude_stream.done`。当前这条 timeout 已移除。
- [已修复] **Claude thinking 只由真实生命周期事件收尾** — 现在只有以下事件会结束 Claude thinking：
  - `control/handler.rs` 在 Claude 发回非空 reply 时发 `Done`
  - `process.rs` / `control/handler.rs` / `daemon/mod.rs` 在 Claude 终端退出、连接断开或强制断开时发 `Reset`
- [结果] “Claude 还在处理但暂时没有终端输出”的场景下，Messages UI 不会再提前消失。

## 验证记录（本轮 #19）

- `cargo test --manifest-path src-tauri/Cargo.toml idle_claude_thinking -- --nocapture`：通过（2 tests）
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（80 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（14 tests）
- `bun test tests/`：通过（21 tests across 4 files）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过

### 20. [已修复] 2026-03-27 Claude / Codex 返回协议新增 `status`

- [已修复] 统一消息结构 `BridgeMessage` 已新增可选 `status` 字段，三态固定为 `in_progress` / `done` / `error`；bridge、daemon、前端类型均已同步。
- [已修复] Claude `reply` tool 已升级为 `reply(to, text, status)`。bridge 对 `status` 做严格校验：缺失值兼容默认成 `done`，非法值直接返回 MCP tool error：`Invalid status: "<value>". Expected "in_progress", "done", or "error".`
- [已修复] Claude channel 转发现在会把 `status` 作为可选 `<channel ... status="...">` meta 透传；Claude system prompt 与 bridge `CHANNEL_INSTRUCTIONS` 也已同步说明该字段。
- [已修复] Codex 最终结构化输出 schema 已扩展为 `{"message","send_to","status"}`。daemon 会解析并保留 `status`；缺失值兼容按 `done` 处理，非法值会写 `error` 级 system log，并生成一条面向用户的错误提示消息。
- [已修复] Claude thinking 的完成条件已切到显式状态驱动：`done` / `error` 会结束 thinking，`in_progress` 不会；空消息仍不渲染，但允许终态空消息只负责结束 thinking。
- [已修复] Codex 侧继续保留流式 `delta.text` 预览；`status` 只在最终结构化完成结果中解析，不要求每个 streaming delta 都带该字段。

## 验证记录（本轮 #20）

- `cargo test --manifest-path bridge/Cargo.toml`：通过（19 tests）
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（85 tests）

### 21. [已修复] 2026-03-27 Claude Code `2.1.85` 已知坏版本前置阻断

- [已修复] **Claude PTY 静默崩溃的根因已定位到上游版本回归** — 现场日志出现 `ERROR _4.useRef is not a function`，错误栈位于 `claude-standalone` 内部 tool activity 渲染函数，而不是 AgentNexus 的 `status` 协议或 PTY 输入链。
- [已修复] **启动前版本校验新增黑名单保护** — `src-tauri/src/claude_cli.rs` 现在除了校验 `>= 2.1.80` 以外，还会额外拒绝 `2.1.85`。这能避免 Claude 以“看似已连接”的状态进入 managed PTY 后再静默炸掉。
- [已修复] **错误提示改成可执行 workaround** — 阻断消息会明确提示当前已知坏版本、对应崩溃形态 `_4.useRef is not a function`，并给出回退命令：
  - `claude install 2.1.84 --force`
  - `npm i -g @anthropic-ai/claude-code@2.1.84`
- [记录] npm registry 复核结果：`2.1.85` 是当前 latest，但 `2.1.84` 仍可获取，因此“前置阻断 + 指向 2.1.84”是当前最稳妥的处理。

## 验证记录（本轮 #21）

- `cargo test --manifest-path src-tauri/Cargo.toml claude_cli`：通过（4 tests）
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（87 tests）
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`：通过

### 22. [已修复] 2026-03-27 非 lead 默认只回 lead 的 prompt 收紧

- [已修复] Claude prompt、bridge `CHANNEL_INSTRUCTIONS`、Codex `baseInstructions` 已统一增加层级路由默认值。
- [已修复] `lead` 仍保留总协调权，可以按上下文直接回复用户或继续分派任务。
- [已修复] 非 `lead` 角色默认只向 `lead` 汇报；只有用户明确点名该身份、或当前指令明确点名目标 worker 时，才允许绕过 `lead` 直接发给 `user` / 指定 worker。
- [目的] 把对用户的默认出口继续收敛到 `lead`，减少 auto/broadcast 场景下 worker 角色直接面向用户发言的概率。

### 23. [已修复] 2026-03-27 角色模型收敛为 lead / coder / reviewer

- [已修复] `tester` 已从当前角色模型中移除，前端 target 选择、agent role 选择、daemon `AGENT_ROLES`、bridge sender/target allowlist、Claude/Codex prompt、Codex `send_to` schema 已同步收口到 `lead / coder / reviewer`。
- [已修复] `reviewer` 现在同时覆盖测试职责：review + test verification。测试结果、验证结论和 review 结论统一由 reviewer 输出。
- [已修复] 角色冲突从“缓存 role 冲突”改成“在线 agent 冲突”：当 Claude 离线时，Codex 现在可以选择 `lead`；但若在线 Claude 已占用 `lead`，Codex 启动前会被直接拒绝。反过来，若在线 Codex 已占用某 role，Claude 连接/启动前也会被阻断，避免 live duplicate role。

### 24. [已修复] 2026-03-27 Claude MCP 注册改为使用真实 claudeRole

- [已修复] `register_mcp()` 之前始终把 `.mcp.json` 写成 `AGENTBRIDGE_ROLE=lead`。这会让 bridge 在 Claude 被切到 `coder` / `reviewer` 后，依旧把 `initialize_result(role)` 和 reply tool 上下文按 `lead` 注入。
- [已修复] 现在 `register_mcp()` 会先从 daemon 读取真实 `claudeRole`，再写入 `.mcp.json`，避免 Claude MCP 调用继续基于错误角色语义运行。
- [结果] Claude 作为 `coder` / `reviewer` 运行时，MCP reply 的角色上下文不再固定成 `lead`。

### 25. [已修复] 2026-03-27 消息气泡颜色改为按真实模型身份显示

- [已修复] 之前消息 UI 直接把 `BridgeMessage.from` 同时当成“路由角色”和“展示身份”，导致 Claude 被切成 `coder` / `reviewer` 后，消息气泡会跟着角色变绿或变黄，看起来像换了模型。
- [已修复] 统一消息结构新增可选 `displaySource` / `display_source`。daemon 现在会保留 `from=lead|coder|reviewer` 作为真实路由角色，同时写入 `displaySource=claude|codex|user|system` 作为 UI 展示身份。
- [已修复] Messages 面板现在只用 `displaySource ?? from` 决定 badge 和颜色；若展示身份与路由角色不同，则额外显示一个次级 role label，例如 `Claude + coder`、`Codex + lead`。
- [结果] 颜色稳定绑定模型身份，角色只作为辅助语义显示，不再把“Claude 的 coder 回复”渲染成 Codex 风格气泡。

### 26. [已修复] 2026-03-27 Codex 恢复启动前模型/智力选择，并强制先选目录

- [已修复] `5f218d2` 曾把 Codex 的 `Reasoning` selector 隐掉，原因是前端当时没有把 effort 真正传到 daemon / app-server。当前这条参数链已重新接通：`CodexPanel -> applyConfig -> daemon_launch_codex -> codex::start -> thread/start(params.effort)`。
- [已修复] `Connect Codex` 之前仍会用 `cwd="."` 自动兜底，因此用户不选目录也能启动。当前前端按钮已改为“未选项目目录时禁用”，store 和 Tauri command 也会在命令边界拒绝空 `cwd`，避免旁路调用继续偷跑。
- [已修复] 模型切换时，Reasoning 会重新回落到该模型的默认 effort；若模型未声明默认值，则退回到首个 supported effort。
- [结果] Codex 现在和 Claude 一样，启动前就能明确选择 `model + reasoning + project`，并且没有选目录就不会发起连接。

### 27. [已修复] 2026-03-30 review 结构清理收口

- [已修复] `src-tauri/src/commands.rs` 与 `src-tauri/src/daemon/codex/handshake.rs` 的内联测试模块已拆到独立文件，分别落在 `src-tauri/src/commands_tests.rs` 与 `src-tauri/src/daemon/codex/handshake_tests.rs`。
- [已修复] 拆分后文件行数重新回到约束内：
  - `commands.rs`: 196 行
  - `handshake.rs`: 183 行
- [已修复] 前端 store 中已经失效的 `launchCodexTui` 接口已彻底移除，包含：
  - `src/stores/bridge-store/index.ts` 中的死实现
  - `src/stores/bridge-store/types.ts` 中的类型声明
- [结果] review 指出的“超出 200 行限制”和“Codex 启动死代码残留”两条问题都已收口。

### 28. [已修复] 2026-03-30 Codex 切角色重连后消息无响应

- [已修复] 现场复现路径为：App 启动 -> Connect Codex -> 断开/切换 Codex 角色 -> 重新 Connect -> 用户发消息后 Codex 无响应。
- [根因] 旧 Codex session 和旧 health monitor 在退出时会无条件执行 `codex_inject_tx = None` 并发出 `agent_status(false)`。当新 session 已经完成握手并接管路由后，旧 session 的迟到清理仍会把新连接的注入通道清空，导致后续消息被当成“Codex 离线”处理并进入 buffer。
- [已修复] daemon state 新增 Codex session epoch。每次启动/失效都会推进 epoch，只有“当前 epoch 的 session”才允许：
  - 挂载 `codex_inject_tx`
  - 在退出时清空 `codex_inject_tx`
  - 发出当前连接的断开副作用
- [已修复] `src-tauri/src/daemon/state.rs` 的权限逻辑已拆到 `state_permission.rs`，本轮新增的 session epoch helper 没有再把 `state.rs` 推回 200 行以上。
- [已修复] 相关文件当前行数：
  - `src-tauri/src/daemon/mod.rs`: 196 行
  - `src-tauri/src/daemon/state.rs`: 187 行
  - `src-tauri/src/daemon/codex/mod.rs`: 200 行
  - `src-tauri/src/daemon/codex/session.rs`: 133 行
- [验证] 新增回归测试 `stale_codex_session_cleanup_cannot_clear_new_session`，锁住“旧 session 不能清掉新 session”这条行为。

## 验证记录（本轮 #28）

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（97 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（19 tests）
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`：通过

### 29. [已修复] 2026-03-27 Codex WS pump loop 稳定化与 session lifecycle 清理

- [已修复] **WS pump loop debug 日志清除** — `src-tauri/src/daemon/codex/ws_client.rs` 中残留的 `/tmp/ws_pump.log` 文件写入、`log` 闭包及所有调试输出已完全移除。pump loop 在生产环境不应写入磁盘日志。
- [已确认] **unsplit WS pump loop 稳定性** — pump loop 使用 `tokio::select!` 在单个 task 中同时处理 outbound（`out_rx.recv()`）和 inbound（`ws.next()`），避免了之前 borrow-split 方案导致的首消息丢失和重连后消息不投递问题。Ping/Pong 由 tungstenite 底层自动处理，无需手动回复。
- [已确认] **session epoch 机制正确性** — `DaemonState` 的 `codex_session_epoch` 计数器通过以下三个原子操作保护：
  - `begin_codex_launch()`: 推进 epoch，返回新值
  - `attach_codex_session_if_current(epoch, tx)`: 只有 epoch 匹配时才挂载 `codex_inject_tx`
  - `clear_codex_session_if_current(epoch)`: 只有 epoch 匹配时才清空 `codex_inject_tx`
  这确保旧 session 退出（无论是正常结束还是 health monitor 检测到进程退出）不会覆盖已经接管的新 session。
- [已确认] **重连后 `codex_inject_tx` 指向当前活跃 session** — `codex::start()` 在 `session::run()` 完成 WS 握手后调用 `attach_codex_session_if_current(launch_epoch, tx)`，如果 epoch 已被新的 launch 推进则挂载失败并清理自身。
- [回归测试] `stale_codex_session_cleanup_cannot_clear_new_session` 锁住了 epoch 竞态保护行为。

## 验证记录（本轮 #29）

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（97 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（19 tests）
- `cargo check --workspace`：通过

## 验证记录（本轮 #17）

- `cargo test --manifest-path src-tauri/Cargo.toml`：通过（78 tests）
- `cargo test --manifest-path bridge/Cargo.toml`：通过（14 tests）
- `bun test tests/`：通过（19 tests across 3 files）
- `cargo clippy --workspace --all-targets -- -D warnings`：通过
- `bun run build`：通过
- `bun run tauri dev` 现场重建后未再复现 `claude-pty-watch` 的 `there is no reactor running` panic

## 相关文档

- `docs/agents/claude-chain.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-channel-api.md`
- `docs/agents/codex-app-server-api.md`
- `docs/agents/codex-app-server-api.zh-CN.md`
