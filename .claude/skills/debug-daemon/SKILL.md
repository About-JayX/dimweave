---
name: debug-daemon
description: 排查 daemon 运行问题。分析日志、端口占用、进程状态。
disable-model-invocation: true
---

排查 AgentBridge daemon 问题：

1. **检查日志**
   ```bash
   tail -100 /tmp/agentbridge.log
   ```

2. **检查进程**
   ```bash
   ps aux | grep -E "(agentbridge|codex.*app-server)" | grep -v grep
   ```

3. **检查端口占用**
   ```bash
   lsof -i :4500 -i :4501 -i :4502 -i :4503 2>/dev/null
   ```

4. **检查 PID 文件**
   ```bash
   cat /tmp/agentbridge-daemon-4502.pid 2>/dev/null
   ```

5. **健康检查**
   ```bash
   curl -s http://127.0.0.1:4502/healthz | jq .
   curl -s http://127.0.0.1:4503/healthz | jq .
   ```

根据以上信息诊断问题并给出修复建议。$ARGUMENTS
