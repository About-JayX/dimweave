# AgentNexus 性能与 UX 重构结果

> 说明：本文件是本轮 `perf-ux-rebuild` 分支的结果摘要，回答“这次具体修了什么”。执行计划仍保留在 `docs/superpowers/plans/2026-04-02-agentnexus-performance-ux-rebuild.md`。

## 范围

本轮工作聚焦两件事：

1. 把长会话下最热的渲染和传输路径收紧，减少不必要的重渲染、列表抖动和 bundle 负担。
2. 把现有“控制面板优先”的界面改成“主时间线优先”的专业 IDE 风格信息架构。

本轮没有改动 Claude `--sdk-url`、Codex app-server、task/session 模型或 bridge-assisted MCP runtime 的核心协议，只在现有架构上做性能和 UX 收口。

## 提交时间线

| 提交 | 主题 | 说明 |
|------|------|------|
| `1995658c` | Stabilize message panel stream rail | 把 Claude/Codex 流式指示器从 Virtuoso 列表项中移出，改成独立 stream rail |
| `3dafaf2e` | Optimize message markdown rendering | 给消息行加 memo 边界，给 markdown 增加 plain-text fast path |
| `209b48a4` | Batch Claude preview stream updates | 修 Claude preview 丢失，并在 Rust 侧给 preview 增加短窗口 batching |
| `1be9a0d3` | Virtualize diagnostics log panel | Logs 改成虚拟化渲染，时间格式统一，去掉高频装饰动画 |
| `fb73b9ec` | Lazy-load markdown renderer | markdown 渲染懒加载，Vite 手动拆 chunk |
| `cdd9f74b` | Isolate shell store subscriptions | 把 shell / timeline / task / agent panel 的订阅边界拆开 |
| `6e506e86` | Reshape shell around the main conversation flow | 改成顶部 context bar、中央时间线、右侧 inspector、底部 secondary drawer |
| `9ab8fda0` | Compress provider controls and surface details on demand | Claude/Codex 面板收成摘要视图，详细配置改为按需展开 |
| `e19a0289` | Quiet Codex startup history noise | provider history 增加 workspace 级 in-flight 去重，Codex history 在 app-server 未就绪时直接走本地 fallback |
| `f418ae99` | Coalesce stream rail updates | 前端把 Claude/Codex 高频流式事件收口成短窗口 flush，去掉 Claude rail 的逐次强制滚动 |
| `c24e80d9` | Soften message and stream surfaces | 消息气泡、badge、Claude/Codex stream rail 改成更轻的草稿态层级 |
| `22b220d3` | Collapse shell header context | header 中间大 task block 改成左侧 compact trigger + task popover，移除长 no-task copy |
| `f780fe05` | Adopt a VS Code-style left activity rail | 引入 VS Code 风格左 rail，右侧常驻区退出主布局 |
| `f1a0e0da` | Route shell nav through a dedicated left rail | 统一 `Task / Agents / Approvals / Logs` 的壳层导航状态机，`Logs` 开始替换主区 |
| `94b17ec0` | Embed the left drawer and preserve pane state | 左侧 drawer 改成嵌入式布局，`Task / Agents / Approvals` 切换或收起后状态保留 |
| `03536b7c` | Add a minimal top bar with workspace context | 顶部收成极简标题 + 当前 workspace，不再承担功能切换 |
| `e59bfadf` | Add shell navigation regression tests | 给 left rail、drawer、logs 主区切换补回归测试 |
| `03d0c0de` | Keep chat-only actions off the logs surface | 去掉 logs 主区里误导性的聊天 `Clear` 操作 |

## 已完成修复

### 1. 流式消息链路更稳定

- Claude/Codex 的 transient stream indicator 不再作为 Virtuoso 的“伪消息行”插入列表。
- 消息总数 `timelineCount` 现在只和真实持久消息绑定，不会因为 stream 开始/结束造成列表跳动。
- Claude preview 文本不再在前端被丢弃，前端会累计并裁剪 preview。
- Rust 侧对 Claude preview `text_delta` 增加了 batching，减少高频 GUI emit。

直接收益：

- 长流式输出时，消息列表更稳，不会额外插入/移除空行。
- Claude preview 可见，而且更新频率更可控。

### 2. 消息渲染热路径更轻

- `MessageBubble` 加了 `React.memo` 边界，不再因为外围 shell 或 stream 状态变化而频繁重渲染。
- `MessageMarkdown` 增加 plain-text fast path，普通文本消息不再走完整 markdown 渲染。
- markdown 渲染输入做了缓存，减少重复解析。
- markdown renderer 被拆成懒加载次级模块。

直接收益：

- 普通对话消息的渲染成本下降。
- markdown 依赖不再全部堵在首屏主 bundle 上。

### 3. Diagnostics 与日志面板可扩展

- Logs 由全量 `.map()` 改成 `Virtuoso` 虚拟化。
- 日志时间显示统一走共享 formatter，不再每行重复构造格式化逻辑。
- 去掉了高频、低价值的持续动画和部分发光效果。

直接收益：

- 长日志滚动更稳。
- logs tab 不再是明显的 DOM 热点。

### 4. Store 订阅边界被拆开

- `App.tsx` 不再直接订阅 bridge store。
- 新增纯 selector，把 shell、task、agents、messages 的读取边界收紧。
- timeline、task inspector、agent 面板和 composer 现在各自订阅最小状态，而不是跟着大 slice 一起刷新。

直接收益：

- 一条消息、一个 stream 更新，不会再带着整层 shell 一起重渲染。
- task/session 变化和聊天流不再高度耦合。

### 5. Shell 信息架构重排

界面最终从“顶部大 header + 分散的面板入口”收口成了：

- 左侧独立 `activity rail`
  - 只保留 `Task / Agents / Approvals / Logs`
  - 同一 icon 再点一次就收起
- 左侧共享 drawer
  - `Task / Agents / Approvals` 共用一个面板容器
  - 切换 pane 或收起后不丢内部状态
- 中央主区域
  - 默认就是聊天页，不再需要 `Conversation` 一级入口
  - `Logs` 是唯一例外，选中后直接替换整个聊天主区
- 顶部极简 bar
  - 只保留 `AgentNexus` 和当前 workspace
  - 不再承担主功能切换，不再像第二个 dashboard

直接收益：

- 主回复链路更清晰，主区默认永远是聊天。
- 功能入口不再分散在顶部、右侧和底部多个区域。
- 侧边导航逻辑和 IDE 习惯更接近，认知负担更低。

### 6. Provider 控件改成 progressive disclosure

- Claude/Codex 面板默认只展示：
  - 当前连接状态
  - role
  - 项目 / 模型 / effort / history 的摘要 chip
  - 主动作按钮
- 详细配置收进 `Details` 折叠区域：
  - model
  - effort / reasoning
  - cwd
  - history picker
- Codex usage 区文案也做了英文归一和状态简化。

直接收益：

- provider 面板不再长期占据大量视觉权重。
- “连接”和“查看/调整配置”的语义被清楚分开。
- provider 配置现在被正确收进左侧共享 drawer，而不是漂在主界面不同位置。

### 7. Composer 更聚焦主动作

- 输入区从“带很多附属信息的小工具栏”改成更单一的主动作区域。
- 目标选择器保留，但视觉权重下降。
- task 信息只保留轻量摘要。
- 发送按钮和快捷键提示保留在明确位置。

直接收益：

- 输入区更像 IDE 的主交互入口，而不是状态杂糅区。

### 8. 基础 UI primitive 的高频过渡被收紧

- `button` 从 `transition-all` 收窄成更具体的 transition 属性。
- `badge` 改成 `transition-colors`。
- `cyber-select` 去掉了更重的 blur 和过宽的过渡范围。
- `ShellContextBar` 去掉了 `backdrop-blur`。
- `AuthActions` 等零散组件也收口到更轻的 transition。

直接收益：

- 高频 hover/focus 面减少无意义的视觉噪声。
- 热表面动画和阴影开销更可控。

### 9. UX 稳定性二次迭代已收口

这轮封板后，又按用户真实反馈继续做了一轮稳定性和体感修复：

- Codex 冷启动历史拉取不再把预期中的 `Connection refused` 直接暴露成红色系统错误。
- workspace 级 provider history 拉取做了 in-flight dedupe，避免 Claude/Codex 面板同时挂载时重复打同一个 history RPC。
- Claude/Codex stream rail 现在先短窗口合并，再写入 Zustand；高频流式状态不再每条事件都触发一次可见渲染。
- Claude rail 去掉了每次 preview 更新都 `scrollTop = scrollHeight` 的路径，减少了长回复时的 sticky scroll 和 layout 抖动。
- 消息气泡、SourceBadge、Claude/Codex 流式卡片都重新降权：accent 保留，但 glow、重边框和“像第二条正式消息”的块感已经被压掉。
- 最终进一步把 `Task / Agents / Approvals` 统一收进左侧共享 drawer，不再保留分散的顶部 header 控件或右侧常驻区。
- `Logs` 现在作为真正的主区模式，而不是会和聊天并排或上下抢空间的 secondary panel。
- 顶部最终只保留极简标题和当前 workspace；没有 active task 时也不再渲染那段长解释文案。
- logs 主区里去掉了只适用于聊天区的 `Clear` 按钮，避免行为歧义。

直接收益：

- 启动日志更安静，错误信号更可信。
- 流式过程更顺，不再靠高频 repaint “假装实时”。
- 主对话、实时工作态、metadata 三层视觉关系更清楚。
- Header 从第二个 dashboard 收回到真正的 status bar。
- 左侧导航、侧边 pane、主区模式三者的职责边界终于稳定。

## 结果指标

### Bundle

在完成 markdown lazy-load、稳定性迭代和最终壳层收口后，主前端 chunk 收敛到：

- `dist/assets/index-CsrEiTuV.js` `353.59 kB`
- `dist/assets/markdown-BTletENE.js` `165.73 kB`
- `dist/assets/MessageMarkdownRenderer-DDkbupb7.js` `2.41 kB`

这意味着 markdown 相关重量依然已经从主启动路径剥离出来，而后续稳定性和 header/popover 改造并没有把 bundle 拉回到早期的膨胀状态。

### 验证

本轮结束时跑过的验证包括：

- `bun test`
- `bun x tsc --noEmit`
- `bun run build`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path bridge/Cargo.toml`
- `bun run tauri dev`

结果：

- `bun test`：`108 passed`
- `cargo test --manifest-path src-tauri/Cargo.toml`：`257 passed`
- `cargo test --manifest-path bridge/Cargo.toml`：`29 passed`
- `bun x tsc --noEmit`：通过
- `bun run build`：通过
- `bun run tauri dev`：能正常编译并启动，daemon 实际监听 `ws://127.0.0.1:4502/ws`

额外 smoke：

- 在 `1400px` 和 `920px` 宽度下分别用浏览器检查了最终布局：
  - 左侧 rail 独立存在
  - `Task / Agents / Approvals` 走左侧 drawer
  - 重复点击当前 icon 会收起
  - `Logs` 会替换整个主聊天区
- 这次 smoke 主要覆盖前端壳层和交互结构；真实的 Tauri `invoke` provider connect 流程仍主要依赖代码回归与原生启动验证，而不是浏览器页面本身。

## 额外修正

在重构过程中还顺手修了两个真实交互问题：

- approvals drawer 初版会在未处理 prompt 存在时反复自动弹回。
- 现在改成只在 permission prompt 从 `0 -> >0` 首次出现时自动展开，用户手动关闭后不会被同一批未处理 prompt 反复打断。
- React 19 下 `TaskPanel` selector 因为返回不稳定空数组触发了 `getSnapshot should be cached` / `Maximum update depth exceeded`，这一点也已经被修掉并补了回归测试。

## 当前已知限制

- 本轮做了真实启动 smoke，也补了前端层的 header/popover 交互 smoke，但没有完成“连接 Claude + 连接 Codex + 长流式回复”这一整套原生窗口人工回放。
- `docs/superpowers/plans/2026-04-02-agentnexus-performance-ux-rebuild.md` 和 `docs/superpowers/plans/2026-04-02-optimization-review-and-diff.md` 仍是未提交的计划文档；它们不是代码改动的一部分。
- 仓库里仍然保留了一些不在当前主路径上的旧组件或样式残留，例如：
  - `TabBtn`
  - `HistoryPicker`
  - `noise-bg`
  - `text-gradient-cyber`
  这些没有进入本轮新 shell 的主交互面，但还没有做 repo 级清场。

## 结论

这轮修复不是单点 patch，而是先把 AgentNexus 从“诊断面板优先”的状态，推进到“主时间线优先”的 IDE 型 agent shell，然后再把第一轮重构暴露出来的噪音和突兀感收口：

- stream 更稳
- markdown 更轻
- logs 可扩展
- shell 订阅边界更清晰
- provider 面板更克制
- composer 更聚焦
- 冷启动噪音更低
- header 更轻
- 气泡和 stream rail 更柔和
- 主 bundle 更小

如果后续还要继续推进，下一阶段最值得做的不是再堆新效果，而是：

1. 做一轮真实 GUI 手工回放和录屏审查。
2. 清理未进入主路径的旧 UI 残留。
3. 再决定是否需要更大范围的 runtime transport abstraction。
