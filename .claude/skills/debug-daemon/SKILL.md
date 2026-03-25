---
name: debug-daemon
description: 排查当前 Rust 内嵌 daemon、bridge sidecar、Codex app-server 的运行问题。
disable-model-invocation: true
---

排查 AgentBridge 当前运行链路时，按下面顺序检查：

1. **检查 Tauri / bridge / Codex 进程**
   ```bash
   ps aux | grep -E "(agent-bridge|agent-bridge-bridge|codex.*app-server|claude)" | grep -v grep
   ```

2. **检查关键端口**
   ```bash
   lsof -i :4500 -i :4502 2>/dev/null
   ```

3. **检查 control server 健康状态**
   ```bash
   curl -s http://127.0.0.1:4502/healthz
   ```

4. **检查项目 MCP 注册**
   ```bash
   cat .mcp.json 2>/dev/null
   ```

5. **检查 bridge sidecar 是否可构建**
   ```bash
   cargo build -p agent-bridge-bridge
   ```

6. **检查前端与 Rust 是否能通过基础校验**
   ```bash
   bun x tsc --noEmit -p tsconfig.app.json
   cargo test
   ```

7. **检查临时 CODEX_HOME 残留**
   ```bash
   ls -la /tmp | grep agentbridge-
   ```

根据以上信息判断故障位于：

- Claude CLI / `.mcp.json`
- bridge ↔ daemon 控制通道
- Codex app-server 启动与 session
- 前端 invoke / listen 链路

不要再按旧 Bun daemon 方式去找 `/tmp/agentbridge.log`、`:4503`、PID 文件或 `daemon/**/*.ts`。$ARGUMENTS
