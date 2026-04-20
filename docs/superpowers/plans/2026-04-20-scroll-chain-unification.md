# 2026-04-20 — Scroll Chain + Dialog Chrome Unification

## Context

滚动相关的 commit + plan 在两周内累积到 36 + 5 份。根因是 `MessageList`
在 Virtuoso 之上又自写了一层"我是否在底部"检测，导致三层决策并存且互相
打架：

1. **Virtuoso 自身** 通过 `followOutput={fn}` 走 follow-on-new-item
2. **DOM scroll 事件监听器** 自算 `distFromBottom > 50` + `scrolledUp`
3. **stream-tail effect** 在每个 delta 上 rAF nudge scrollTo(scrollHeight)

为了屏蔽 Virtuoso 自身 layout 校正产生的 false "scrolled up" 事件，又
引入了 **300ms programmatic-scroll immunity 窗口**，通过时间戳判断当前
scroll 事件是不是我们自己触发的。Immunity 窗口本身脆弱（跨浏览器 render
pipeline、跨 measure 周期都可能超 300ms），每改一次就引入新 regression。

同时，两个最常用的 Dialog（`ProviderAuthDialog` 和 `TaskSetupDialog`）
各自硬编码 `max-h-[90vh]` / `overflow-hidden` / shrink-0 header/footer，
漂移一次就一个 commit（`8458800`、`454ee30`）。

## 设计决定

### 滚动：单一 latch + 事件语义分层

不再把 `atBottomStateChange(false)` 当作"用户滚走了"的信号。那是 Virtuoso
在报告"相对位置不在底部阈值内"，content growth（新消息、Footer 增高）也
会瞬间让 `scrollTop + clientHeight < scrollHeight`，触发 false。

正确的信号来自**用户输入事件**——Virtuoso 的内部 scrollTo 不会触发这些：

- `wheel` 且 `deltaY < 0`：滚轮/触控板向上滚
- `touchmove`：手指在 scroller 上移动
- `keydown` `ArrowUp` / `PageUp` / `Home`：键盘导航

这三个事件任意触发 → 设置 `userAwayRef = true`。
反向清除来自 **`atBottomStateChange(true)`**——Virtuoso 明确告诉我们
"现在又进底部阈值了"，只此一处清 latch。

### useScrollAnchor hook

`src/components/MessagePanel/use-scroll-anchor.ts` 新增。所有"是否跟底部"
决策集中在此，暴露：

- `virtuosoRef` / `scrollerRefCallback`：透传给 Virtuoso
- `onAtBottomStateChange(bottom)`：auto-clear 入口
- `followOutputMode()`：`searchActive ? false : userAway ? false : "smooth"`
- `scrollToBottom()`：pill 点击入口，清 latch + 动画滚动
- `nudgeToBottom()`：stream-tail effect 用，内部自带 `searchActive` /
  `userAway` 守卫；caller 只要在 delta 变化时 rAF 调它
- `showBackToBottom`：派生显示状态（也受 `searchActive` 屏蔽）

### MessageList 重构

从 262 行压到约 150 行：

- 删除 `stickyRef`、`programmaticScrollRef`、`didInitialScrollRef` 等本地
  ref 状态（全部迁入 hook）
- 删除 `useEffect` 里的 DOM scroll 监听器（hook 里按输入事件改写）
- 删除 initial scroll effect（迁入 hook）
- stream-tail effect 简化为"如果有 stream，就每帧 `nudgeToBottom()`"
  ——条件判断不再在 effect 内，下沉到 `nudgeToBottom` 内部

### view-model 清理

删除：

- `shouldClearStickyOnScroll()`：替代机制（input-event-driven latch）
- `PROGRAMMATIC_SCROLL_IMMUNITY_MS`：immunity 窗口废弃
- `shouldScrollOnDraftStart()`：与 `shouldScrollOnStreamTail` 语义重合
- `getMessageListFollowOutputMode()`：移入 hook
- `shouldResetMessageListInitialScroll()`：移入 hook

保留：

- `STICKY_BOTTOM_THRESHOLD = 50`：传给 Virtuoso `atBottomThreshold`
- `shouldScrollOnStreamTail(...)`：单纯纯函数，谁写判断条件都能复用

### DialogLayout 抽象

`src/components/ui/dialog-layout.tsx` 新增。拥有：

- 外层 overlay + backdrop click
- ESC 关闭
- `max-h-[90vh]` 默认高度（可覆盖）
- 固定 header（`border-b`）
- 滚动 body（`flex-1 min-h-0 overflow-y-auto`；传 `bodyFlex` 时改为
  `flex overflow-hidden` 由 caller 控制内嵌 overflow）
- 固定 footer（`border-t`，可选）

宽度 preset：`sm/md/lg` 对应 `max-w-md/max-w-lg/max-w-2xl`。

### 迁移

- `ProviderAuthDialog`：外壳全删，wraps DialogLayout(width="md")
- `TaskSetupDialog`：外壳全删，wraps DialogLayout(width="lg", bodyFlex)
- `TaskSetupDialogSkeleton`：同上，统一外壳样式

三个 Dialog 现在共用同一套 overlay / 高度 / overflow 语义。以后要改
dialog 统一行为（动画、ESC 行为、键盘焦点管理）在一处动。

## 关键文件

| 文件 | 角色 |
|---|---|
| `src/components/MessagePanel/use-scroll-anchor.ts` | 新：hook 集中所有 scroll latch |
| `src/components/MessagePanel/MessageList.tsx` | 重构：262→~150 行，消费 hook |
| `src/components/MessagePanel/view-model.ts` | 清理：删除重复 helper |
| `src/components/MessagePanel/view-model.test.ts` | 清理：删除旧 helper 的测试 |
| `src/components/MessagePanel/MessageList.test.tsx` | 清理：删 `getDraftScrollStrategy` 测试 |
| `src/components/ui/dialog-layout.tsx` | 新：DialogLayout 组件 |
| `src/components/ToolsPanel/ProviderAuthDialog.tsx` | 迁移到 DialogLayout |
| `src/components/TaskPanel/TaskSetupDialog.tsx` | 迁移到 DialogLayout |
| `src/components/TaskPanel/TaskSetupDialogSkeleton.tsx` | 迁移到 DialogLayout |
| `src/components/TaskPanel/TaskSetupDialog.test.tsx` | 删除 history-ellipsis 测试（session 改 form variant） |

## 验证

- `bun test src/components/MessagePanel/ src/components/TaskPanel/ src/components/ToolsPanel/` — **175/177 pass**。
  失败 2 个都是 pre-existing `bg-transparent` 按钮样式断言，commit
  `642ac2d` 改样式时漏更新，与本轮无关。
- `bun x tsc --noEmit -p tsconfig.app.json` — 本轮文件无新 error（其它
  pre-existing error 跟本轮无关）。

## 对照原 review 的根因

| 原根因 | 消除方式 |
|---|---|
| 双 scroll-anchor 策略赛跑 | `nudgeToBottom` 单策略，优先 `scroller.scrollTo(scrollHeight)`，回退 `virtuoso.scrollToIndex` |
| 300ms immunity gate 脆弱 | 整个删除，不再需要 |
| 三层独立决策无 owner | 全部收口到 `useScrollAnchor` 内部 ref + 派生函数 |
| 手写 sticky 距离算法 | 删除，改用 Virtuoso `atBottomThreshold` + input-event latch |
| Task 切换 state 未隔离 | hook 实例 per-mount，新 mount 默认 `userAway=false` |

## 明确不做

- ❌ 再抽 `MessagePanel` 的 logs 滚动到同一 hook：logs 面板用的是 Virtuoso
  自己的 atBottom 机制 + followOutput prop，本身没有自写 sticky 层，不在
  "反复修" 的范围。
- ❌ 把 TaskContextPopover / ToolsPanel 的 `overflow-y-auto` pane 也改
  hook 管理：这些是静态布局 overflow，不参与消息时间线的 follow-bottom
  语义。
- ❌ 恢复 `history` variant 的 middleEllipsis 行为到新 form variant：
  form variant 通过 CSS `truncate` 处理；长 session 标题前端可读性尚可，
  不引入多余 pure-fn。

## CM (Configuration Management)

### Commit
- **Hash**: `(will be filled after commit)`
- **Subject**: `refactor(ui): unify scroll anchor + dialog chrome`
- **Scope**: 10 files (2 new, 8 modified). 消除 36 个 scroll 相关 commit
  的根因：三层决策 → 单 latch；两个 Dialog 重复 chrome → 统一组件。
