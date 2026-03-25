---
name: add-adapter
description: 为当前 Rust 架构添加新的 agent 接入能力（不是旧 Bun daemon 适配器）。
disable-model-invocation: true
argument-hint: <agent-name>
---

为 `$ARGUMENTS` 添加新的 agent 接入时，按当前架构执行：

1. **先判断接入形态**
   - 如果新 agent 通过 MCP / stdio 接入，优先参考 `bridge/`
   - 如果新 agent 通过本地进程 + WebSocket / IPC 接入，优先参考 `src-tauri/src/daemon/codex/`

2. **更新 Rust daemon 状态与协议**
   - 检查 `src-tauri/src/daemon/state.rs`
   - 检查 `src-tauri/src/daemon/types.rs`
   - 检查 `src-tauri/src/daemon/routing.rs`
   - 为新 agent 定义上线 / 下线 / 路由规则

3. **如果需要新的 bridge sidecar 能力**
   - 修改 `bridge/src/tools.rs`
   - 修改 `bridge/src/mcp.rs`
   - 修改 `bridge/src/daemon_client.rs`
   - 保证 `bridge/src/types.rs` 与 daemon 协议一致

4. **如果需要新的本地 session 类型**
   - 参考 `src-tauri/src/daemon/codex/`
   - 新增 lifecycle / session / handler 分层
   - 不要把所有逻辑塞进一个文件

5. **更新前端**
   - `src/stores/bridge-store/index.ts`
   - `src/types.ts`
   - `src/components/AgentStatus/`
   - 让 UI 能显示新 agent 的状态与消息

6. **更新文档**
   - `CLAUDE.md`
   - `.claude/rules/architecture.md`
   - 对应 rules 文件

7. **校验**
   - `bun x tsc --noEmit -p tsconfig.app.json`
   - `cargo test`

不要再创建 `daemon/**/*.ts`、`gui-server.ts`、`control-server.ts`、`bridge.ts` 这类旧 Bun 架构文件。
