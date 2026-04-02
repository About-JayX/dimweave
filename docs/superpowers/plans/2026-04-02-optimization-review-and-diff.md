# 优化文档审阅与改进方案

> 针对 `2026-04-02-agentnexus-performance-ux-rebuild.md` 的逐条审阅，结合代码审计结果。

---

## Diff 总表

| # | 原文档 | 问题/改进 | 本文方案 | 类型 |
|---|--------|----------|---------|------|
| 1 | WS1-S1: "Replace many emit_* with dispatcher" | 当前只有 6 个 emit 函数，不是"many"。问题不在函数数量，在于 stream delta 的调用频率（每个 text_delta 1 次 emit，1000 char 回复 ≈ 1000 次） | Rust 侧 stream delta 批量合并（50ms 窗口），其他 emit 保持原样 | **修正** |
| 2 | WS1-S2: "Tauri Channel 替代 listen" | 方向正确但 scope 过大。当前 6 种事件只有 `claude_stream` 和 `codex_stream` 是高频，其余（agent_message, agent_status, permission_prompt, system_log）是低频 | 只对 `claude_stream` / `codex_stream` 改用 Tauri Channel；其余保持 `emit` | **缩小范围** |
| 3 | WS1-S3: "Define stable runtime event types" | 过度设计。当前事件类型已经稳定（6 种），引入 `RuntimeUiBatch` tagged union 增加复杂度但不解决实际瓶颈 | 保持现有事件类型，不引入新抽象层 | **删除** |
| 4 | WS2-S1: "Split bridge store by behavior" | 方向正确。当前 `BridgeState` 混合了 shell/chat/stream/diagnostics 四类状态 | 拆分为 `useChatStore`（messages）、`useStreamStore`（claude/codex stream）、`useShellStore`（agents/roles/connected）。permission 留在 bridge-store | **保留+细化** |
| 5 | WS2-S2: "ids + byId normalized messages" | 正确但优先级偏低。当前 messages 数组在 200 条内，array clone 成本不是主瓶颈 | 延后。先解决 stream indicator 和 markdown 的高频渲染问题 | **降优先级** |
| 6 | WS2-S3: "Stream indicators 不放 totalCount" | **这是最高优先级修复。** 审计确认：stream indicator 在 `getTransientIndicators()` 里按 `thinking` 状态切换，导致 Virtuoso `totalCount` 在每次 stream 开始/结束时 ±1，触发全列表 re-layout | stream indicator 从 Virtuoso 列表中移出，放到 Virtuoso 下方的固定位置 `div`。不参与 totalCount | **保留+提升优先级** |
| 7 | WS2-S4: "Markdown fast path" | 正确。审计确认 `MessageMarkdown` 无 memo，每次 parent re-render 都重新解析 markdown | 1) `React.memo(MessageBubble)` 按 `msg.id` 浅比较。2) `MessageMarkdown` 加 `useMemo` 按 `content` 缓存。3) 纯文本（无 `#*[]` 等标记）直接渲染 `<pre>` 跳过 react-markdown | **保留+具体化** |
| 8 | WS2-S5: "Virtualize diagnostics" | 正确。审计确认 logs 用 `.map()` 渲染最多 200 条，无 virtualization | 复用 `react-virtuoso` 的 `Virtuoso` 组件渲染 logs tab | **保留** |
| 9 | WS2-S6: "Code splitting" | 正确。`vite.config.ts` 无任何 chunk 策略。react-markdown 始终加载 | `React.lazy` 包裹 `MessageMarkdown`。`vite.config.ts` 加 `manualChunks` 分离 react-markdown + remark-gfm | **保留+具体化** |
| 10 | WS2-S7: "React concurrency" | 方向正确但缺乏具体 target。`startTransition` 不能用在 Zustand selector 上 | `useDeferredValue` 包裹 `previewText`（stream indicator 的高频文本）。history 加载用 `startTransition` | **保留+具体化** |
| 11 | WS3: "Redesign shell" | 方向正确但 scope 过大（全盘重排 layout）。当前 layout 基本可用，问题是 TaskPanel 始终渲染 + stream 指示器导致的 churn | 不做全盘重排。只做：1) TaskPanel 默认折叠（点击展开）。2) ClaudePanel/CodexPanel 折叠为 status bar。3) 保持当前 layout 骨架 | **缩小范围** |
| 12 | WS4: "Replace visual noise" | 方向正确。审计确认 `gradient-shift` 6s 无限循环、7 个 `backdrop-blur`、17 个 `transition-all` | 删除 `gradient-shift` 无限动画。`backdrop-blur` 从 7 处减到 2 处（只保留 dropdown overlay）。`transition-all` 全部替换为 `transition-colors` 或 `transition-opacity` | **保留+量化** |
| 13 | WS5: "Verify performance" | 过于笼统。"Add runtime dispatcher tests" 但 dispatcher 本身不该引入（见 #1） | 具体测试：1) stream delta 批量合并的窗口验证。2) Virtuoso 不因 stream 触发 re-layout。3) 200 条 log Virtuoso 渲染 < 16ms。4) bundle size 对比 | **具体化** |
| 14 | 原文档未涉及 | Claude SDK stream 每个 text_delta 触发 1 次 `gui::emit_claude_stream(Preview)`，高频 stream 场景下 ~1000 次/秒 | Rust 侧加 50ms 批量窗口：缓冲 text_delta，每 50ms 合并发一次 `Preview { text: accumulated }` | **新增** |
| 15 | 原文档未涉及 | `ClaudeStreamIndicator` 的 `previewText.slice(-3000)` 每次渲染都重新计算 | `useMemo(() => previewText.slice(-3000), [previewText])` | **新增** |
| 16 | 原文档未涉及 | logs 中 `new Date(l.timestamp).toLocaleTimeString()` 每条每次渲染都实例化 Date | 预格式化 timestamp 在 store 写入时完成，或用 `Intl.DateTimeFormat` 单例 | **新增** |
| 17 | 原文档未涉及 | `listener-setup.ts` 的 `handleClaudeStreamEvent` preview case 当前被 revert 为 `return {}` — stream 文本不会累积到前端 | 恢复 preview 累积逻辑（之前已实现但被 revert） | **新增（bug fix）** |
| 18 | 原文档未涉及 | `CodexStreamIndicator` 订阅整个 `codexStream` slice（7 个字段），任何字段变化都 re-render | 拆分为 `useBridgeStore(s => s.codexStream.thinking)` + 单独订阅需要的字段 | **新增** |
| 19 | 原文档 WS1-S4: "Implement batching and caps" | 方向正确但没说 batch 什么、cap 什么 | 具体方案：stream delta → 50ms batch（Rust 侧）。system_log → 内存 cap 200 条（已有）。stream previewText → 前端 cap 5000 chars | **具体化** |
| 20 | 原文档假设 "Claude stays on --sdk-url" | 正确但遗漏：SDK 路径当前仍注入 bridge MCP（`build_agentnexus_mcp_config`），bridge sidecar 仍然是 3 进程模型的一部分。优化 stream 时必须考虑 bridge WS 也在用同一个 event loop | 不假设 bridge 会被移除。优化方案必须兼容 bridge + SDK 共存 | **修正假设** |

---

## 执行优先级（按 ROI 排序）

### P0: 立即修复（最高收益/最低成本）

| 序号 | 改动 | 预期效果 | 改动量 |
|------|------|---------|-------|
| 6 | Stream indicators 移出 Virtuoso totalCount | 消除 stream 期间的 Virtuoso re-layout | ~30 行前端 |
| 7 | MessageBubble + MessageMarkdown 加 memo | 消除 unrelated parent render 导致的 markdown 重解析 | ~10 行前端 |
| 14 | Rust stream delta 50ms 批量 | 1000 次 emit/s → 20 次 emit/s | ~30 行 Rust |
| 17 | 恢复 preview 累积逻辑 | stream 文本实际显示在前端 | ~5 行前端 |

### P1: 短期优化

| 序号 | 改动 | 预期效果 | 改动量 |
|------|------|---------|-------|
| 8 | Logs 用 Virtuoso | 200 DOM nodes → ~15 visible | ~40 行前端 |
| 12 | 删除 gradient-shift + 减少 backdrop-blur | 减少 GPU compositing | ~20 行 CSS |
| 15 | previewText.slice 加 useMemo | 减少 string 操作 | ~3 行 |
| 16 | 预格式化 log timestamp | 减少 Date 实例化 | ~10 行 |
| 18 | Codex stream indicator 细粒度订阅 | 减少无关 re-render | ~5 行 |

### P2: 中期重构

| 序号 | 改动 | 预期效果 | 改动量 |
|------|------|---------|-------|
| 4 | 拆分 bridge-store | 减少 cross-domain re-render | ~200 行前端 |
| 9 | react-markdown lazy load + vite chunk | 初始 bundle 减小 ~80KB | ~20 行配置 |
| 11 | TaskPanel 默认折叠 | 减少 always-visible 高频组件 | ~50 行前端 |
| 10 | useDeferredValue for previewText | 降低 stream indicator 渲染优先级 | ~5 行 |

### 不执行

| 序号 | 原文档项 | 理由 |
|------|---------|------|
| 1 | 引入 runtime UI dispatcher | 当前 6 个 emit 函数足够，问题在频率不在架构 |
| 3 | RuntimeUiBatch tagged union | 过度抽象，不解决实际瓶颈 |
| 5 | ids + byId 消息规范化 | 200 条消息 array 不是瓶颈 |
| 11 | 全盘 layout 重排 | 当前 layout 可用，改动 ROI 不合理 |

---

## 与原文档的根本分歧

### 1. 瓶颈定位

**原文档：** "The app shell subscribes to broad bridge store slices" — 暗示问题在 store 架构。

**审计结论：** 实际瓶颈在三个具体位点：
1. Virtuoso totalCount 因 stream indicator 变化而 re-layout
2. react-markdown 无 memo 导致每次 parent render 重解析
3. Rust 每个 text_delta 发一次 emit（~1000 次/秒）

Store 拆分有用但不是最高优先级。

### 2. 改造深度

**原文档：** 5 个 workstream 全面重构（runtime transport + state + shell + visual + tests），scope 接近重写。

**本文方案：** 10 个精确的点修复覆盖 80% 收益，总改动量 < 500 行。全面重构留到 P2。

### 3. 对 SDK 链路的理解

**原文档：** 假设 bridge 已移除，Claude 是纯 SDK 直连。

**实际：** bridge MCP 仍被注入（`build_agentnexus_mcp_config`），3 进程共存。优化不能假设单一传输层。

---

## 验证标准

| 检查项 | 当前状态 | 优化后目标 |
|--------|---------|-----------|
| Stream 期间 Virtuoso re-layout | 每次 indicator 出现/消失触发 | 零次（indicator 不在 Virtuoso 内） |
| Stream 期间 Rust emit 频率 | ~1000 次/秒（每 char 一次） | ~20 次/秒（50ms 批量） |
| MessageBubble re-render | parent 任何更新都触发 | 只在 `msg.id` 变化时触发 |
| react-markdown 解析 | 每次 render 都解析 | 仅 `content` 变化时解析 |
| Logs DOM 节点 | 200 个 | ~15 个（visible only） |
| 初始 bundle size | 870KB (单 chunk) | ~600KB main + ~270KB lazy |
| gradient-shift 动画 | 6s 无限循环 | 移除或 hover-only |
| backdrop-blur | 7 处 | 2 处 |

```bash
# 验证命令
cargo test
bun x tsc --noEmit
bun run build    # 检查 chunk 分布
# 手动：连接 Claude → stream 长回复 → 观察帧率和 CPU
```
