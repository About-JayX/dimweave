# Tools → Accounts 面板 + Claude profile/usage

## 目标

Tools 侧边栏在 Telegram / 飞书 之上新增 `Accounts` section，默认展开，展示 Claude 和 Codex 的账号信息 + 实时用量。

## 背景

- 之前只有 `MobileInspectorSheet` 里挂 `AuthActions`，桌面端没入口。
- Codex 侧已有 `profile` + `usage`（`get_codex_account` / `refresh_usage`）；Claude 侧完全没有用户面板。
- 用户想看的是 **"我当前登录的是谁、用量还剩多少"**，而不是运行时 agent 状态。

## 实现

### 1. Claude Profile

- 新 `src-tauri/src/claude/profile.rs`：调 `GET /api/oauth/profile`（OAuth bearer）返回 `ClaudeProfile { email, displayName, subscriptionTier, rateLimitTier, organizationName, subscriptionStatus }`
  - `subscriptionTier` 从 `has_claude_max` / `has_claude_pro` 推导（`max | pro | free`）
- Tauri command `get_claude_profile`
- 5 条解析单测（max/pro/free、缺字段 default、malformed JSON）

### 2. Claude Usage (5h / 7d 窗口)

- `/api/oauth/usage` 只暴露 `extra_usage`，**不是**订阅 rate-limit。真正的 5h/7d 在 `POST /v1/messages` 响应 header 里：
  - `anthropic-ratelimit-unified-5h-utilization` / `-reset` / `-status`
  - `anthropic-ratelimit-unified-7d-utilization` / `-reset` / `-status`
- 新 `src-tauri/src/claude/usage.rs`：`get_usage()` 发 `max_tokens=1` 的 Haiku ping，从响应 header 抓指标
  - 成本可忽略但非零 → UI 用 **手动 Refresh 按钮**，不 mount 时自动拉
- Tauri command `get_claude_usage`

### 3. UI

- `src/stores/claude-account-store.ts`：新 store 镜像 `useCodexAccountStore` 的 profile/models/usage pattern
- `src/components/ToolsPanel/AccountsInfoPanel.tsx`：Claude / Codex 两张卡
  - Claude 卡：MAX/PRO/FREE 徽章 + email + name + rate_limit + 5h/7d MiniMeter（订阅活跃状态着色）
  - Codex 卡：plan 徽章 + email + name + primary/secondary MiniMeter + Refresh 按钮
- `src/components/ToolsPanel/index.tsx`：新 `<DisclosureSection title="Accounts">`（默认展开）
- 短暂存在的 `AgentsInfoPanel`（展示 active task 的 agent runtime）被 `AccountsInfoPanel` 取代：用户想要的是账号级信息

## 文件清单

| 文件 | 作用 |
|---|---|
| `src-tauri/src/claude/profile.rs` + tests | `/api/oauth/profile` 拉 + 解析 |
| `src-tauri/src/claude/usage.rs` + tests | `POST /v1/messages` ping + header 解析 |
| `src-tauri/src/main.rs` | 注册 `get_claude_profile` / `get_claude_usage` 两个 command |
| `src/stores/claude-account-store.ts` | Zustand store（profile/models/usage） |
| `src/components/ToolsPanel/AccountsInfoPanel.tsx` | 两张卡 UI |
| `src/components/ToolsPanel/index.tsx` | Accounts section 接入 |

## 不做的事

- 不把 Claude usage 做成自动轮询（成本 > 信号价值）
- 不在 Accounts 里塞登录操作（后续 `Provider Authentication dialog` 统一处理；见 [2026-04-19-provider-auth-dialog.md](2026-04-19-provider-auth-dialog.md)）

## 验证

- `cargo test` — 704 + 新 profile/usage 单测
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：重启 dev → Tools → Accounts → 看到邮箱 + tier + 点 Check usage 后出现 MiniMeter

## CM 回填区

- `b06379ad` — `feat(tools): Agents section shows active task's agent runtime info` — 初版展示 per-task agent 运行状态；很快被账户信息方向取代
- `d7faefb5` — `feat(claude): fetch account profile via /api/oauth/profile` — Rust `claude::profile` + 5 条 parser 单测
- `fe32dc58` — `feat(tools): Accounts section shows Claude and Codex account info` — `AccountsInfoPanel` 替换 `AgentsInfoPanel`，两张卡并列
- `e288286c` — `feat(claude): show 5h/7d usage meters from rate-limit response headers` — `claude::usage` + `Check usage / Refresh` 按钮 + MiniMeter
- `10d352b0` — `fix(tools): surface Codex auth actions inside Accounts panel` — Codex AuthActions 从 mobile sheet 搬进桌面 Accounts 卡（过渡态，后续被 Provider Authentication dialog 再次搬走）
