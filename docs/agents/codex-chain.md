# Codex 链路修复记录

> **强制规则:** 每次修复或发现 Codex 链路问题，必须在此文档记录。
> 包括：问题描述、根因、修复方案、运行时验证结果。
> 未修复的问题也必须记录，标注 `[未修复]` 和原因。

## 官方文档参考

- 完整 API: `docs/agents/codex-app-server-api.md`
- 在线: https://developers.openai.com/codex/app-server
- **注意: 官方文档与 CLI 实现存在多处不一致，以运行时测试为准！**

## 协议对照与修复记录

### 2026-03-25: 初始协议审计

#### [已修复] 缺少 `initialized` 通知

**问题:** 官方文档要求 `initialize` 响应后必须发送 `{ "method": "initialized", "params": {} }`。
当前实现没有发送，导致 app-server 不继续处理后续请求。

**修复:** `session.rs` 收到 init response 后发送 `initialized` 通知。

**验证:** 运行时测试确认握手成功。

#### [已修复] dynamicTools schema 字段名 — 文档与实现不一致

**问题:** 官方文档写 `parameters`，但 Codex CLI 实际要求 `inputSchema`。
报错: `Invalid request: missing field 'inputSchema'`

**根因:** 官方文档与 CLI 实现不一致。

**修复:** 保持 `inputSchema`。曾错误改为 `parameters`，验证失败后改回。

**教训:** 官方文档不可信，必须运行时测试验证。

#### [已修复] sandbox 值格式 — 全局统一 kebab-case

**问题:** 三次修复才找到正确方案。

| 尝试 | 方案 | 结果 |
|------|------|------|
| 1 | `roles.rs` 全改 camelCase | config.toml 报错 `unknown variant 'workspaceWrite'` |
| 2 | `roles.rs` kebab, `session.rs` 转 camelCase | `thread/start` 报错 `unknown variant 'workspaceWrite'` |
| 3 | 全部 kebab-case，不做转换 | 成功 |

**结论:** Codex CLI 全部使用 kebab-case (`workspace-write`, `read-only`)，包括 JSON-RPC `thread/start` 的 `sandbox` 参数。与官方文档的 camelCase 描述完全相反。

**验证:** `bun` 脚本直接测试 `inputSchema` + kebab-case → `thread/start` 成功。

#### [已修复] `--config` CLI flags 格式

**验证:** `--config sandbox_mode="workspace-write"` 格式正确。

### 2026-03-25: 生命周期问题

#### [已修复] stop→start 竞态 — 端口未释放

**问题:** Disconnect 后立即 Connect，新 codex 进程报 `Address already in use (os error 48)`。

**根因:** `lifecycle::stop()` kill 进程后，OS 需要时间释放端口 4500。新进程立即启动时端口仍被占。

**修复:**
1. `lifecycle::stop()` kill 后等 500ms 端口释放
2. `codex::start()` 启动前轮询端口空闲（最多 5s）

#### [已修复] Codex 孤儿进程 — PPID=1

**问题:** Disconnect 后 `codex app-server` 进程仍然存活，PPID=1（已脱离进程树）。

**根因:** Codex CLI 内部 fork/exec 真正的 app-server。`kill_on_drop(true)` 和 `start_kill()` 只能 kill 直接子进程，不能 kill 孙进程。

**修复:** `lifecycle::stop()` 增加 `kill_port_holder()` — 用 `lsof -ti:{port}` 找到端口占用进程并 SIGKILL。

**运行时验证:** Connect→Disconnect→Connect 循环成功。日志显示 `[Codex] killing orphan process {pid} on port 4500`。

#### [已修复] agent_status(true) 在握手完成前发出

**问题:** `codex::start()` spawn session 后台任务后立即 emit `agent_status(true)`，但此时握手（initialize→initialized→thread/start）尚未完成。前端显示 Connected 但 thread ID 还没拿到。

**修复:** `session::run()` 接受 `ready_tx` oneshot，握手成功后发送 thread ID。`codex::start()` 等待 `ready_rx` 收到 thread ID 后才 emit `agent_status(true)`。

#### [已修复] 握手失败资源泄漏

**问题:** 当 `session::run()` 握手失败（返回空 thread ID）时，`codex::start()` bail 但未清理：
- 健康监控任务继续运行（孤儿 task）
- 子进程未被 kill（Arc 引用计数 > 0）
- 临时目录未清理

**修复:** 失败路径增加: `cancel.cancel()` + `lifecycle::stop(&mut child)` + `cleanup_session()`。

#### [已修复] CODEX_HOME 在进程仍引用时被删除

**问题:** `CodexHandle::stop()` 中 `cleanup_session()` 删除 `/tmp/agentbridge-{pid}-{id}/`，但旧 codex 进程可能还在读取该目录下的文件。新 session 的 `thread/start` 报错: `CODEX_HOME points to "/tmp/agentbridge-...", but that path does not exist`。

**根因:** stop 删目录 → start 创建新 session 用新 ID → 但旧进程引用的目录已被删。这发生在端口还没释放、新进程复用了旧 CODEX_HOME 的路径时。

**修复:** 每次 start 用独立的 session ID（递增），stop 时先 kill 进程再删目录，加端口释放等待。

### 2026-03-25: 深度审查补充修复

#### [已修复] pre-init buffer replay break 不传播

**问题:** `bridge/mcp.rs` 中 pre-init 消息回放时，`write_line` 失败的 `break` 只退出 `for` 循环，不退出外层 `loop`。MCP task 在 stdout 损坏后继续运行。

**修复:** 增加 `replay_ok` flag，`for` 循环后检查并 `break` 外层循环。

## 待确认项

#### [待确认] `settings.developer_instructions` 有效性

**问题:** 当前把 `developer_instructions` 放在 `params.settings.developer_instructions`。官方文档未明确此字段。

**状态:** 保持当前实现，等运行时有 Codex 响应后验证。

#### [待确认] tool response 格式

**问题:** handler.rs 回复格式:
```json
{ "id": id, "result": { "contentItems": [{ "type": "inputText", "text": "..." }], "success": true } }
```
需确认是否与 Codex 期望的 dynamic tool call response 格式匹配。

**状态:** 需运行时测试验证。

## 当前已知限制

- 端口 4500 固定，不可配置
- `kill_port_holder` 用 SIGKILL 可能误杀同端口的其他进程
- 不处理 `turn/completed` 通知
- 不处理 `item/agentMessage/delta` 流式文本
- 不处理 `item/commandExecution/requestApproval` 审批
- 不处理 `-32001` 过载错误重试
- 健康监控和 session task 独立退出时会双重 emit `agent_status(false)`
