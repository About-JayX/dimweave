# Claude 订阅登录接入 Provider Authentication Dialog

## 目标

`claude auth login / logout / status` 接入 dialog，UI 和 Codex 侧对称：一键触发浏览器 OAuth，轮询登录状态，Logout 按钮可回收。

## 背景

- Claude CLI 原生支持 `claude auth login / logout / status`（2.1.107 起）
- `claude auth status` 返回结构化 JSON：`{ loggedIn, authMethod, apiProvider, email, orgId, orgName, subscriptionType }`
- `claude auth login` 启动时打印 `If the browser didn't open, visit: <URL>` —— 可以 regex 抓
- Provider Authentication dialog 初版里 Claude 侧写的是 `Subscription is managed via Claude CLI. Run claude login in a terminal to sign in.` —— 需求升级为"和 Codex 一样能点按钮"

## 实现

### 1. Rust: `claude::auth` 模块

- `src-tauri/src/claude/auth.rs`：复用 `codex::oauth_helpers::{pump_stream, LoginState, StreamEvent, parse_verification_uri}`（Codex 侧已经 generic）
- `ClaudeAuthHandle`：内部 `Mutex<Option<oneshot::Sender<()>>>` 用于取消正在跑的 `claude auth login` 子进程
- 3 个异步函数：
  - `start_login(handle)` — spawn `claude auth login`、抓 URL、5s deadline、挂 cancel hook
  - `do_logout()` — `claude auth logout` status check
  - `get_status()` — `claude auth status` → serde parse `ClaudeAuthStatus`

### 2. Tauri commands + app state

- `src-tauri/src/main.rs`：4 个新 command `claude_login / claude_cancel_login / claude_logout / claude_auth_status`
- `tauri::Builder::default().manage(Arc::new(ClaudeAuthHandle::new()))` 挂到 app

### 3. TS store

- `useClaudeAccountStore` 扩展：
  - state: `authStatus` / `loginPending` / `loginUri` / `loginError`
  - actions: `login` / `cancelLogin` / `logout` / `fetchAuthStatus`
- `login()` 流程：invoke → set loginUri → 每 2s 轮询 `claude_auth_status` → 看到 `loggedIn && email` 就 clearLoginPolling + fetchProfile + fetchModels
- 3 分钟 safety timeout

### 4. Dialog UI

- `ProviderAuthDialog.tsx::SubscriptionRow`（Claude 分支）：
  - `loginPending` → spinner + "Open login page →" + Cancel
  - 已登录 → email + MAX/PRO 徽章 + Logout
  - 未登录 → "Login with Claude Max" 主按钮
- Dialog open 时 `void fetchClaudeAuthStatus()` 同步 Logged-in 状态

## 文件清单

| 文件 | 改动 |
|---|---|
| `src-tauri/src/claude/auth.rs` | 新模块（3 函数 + handle） |
| `src-tauri/src/claude/mod.rs` | `pub mod auth` |
| `src-tauri/src/main.rs` | 4 个 command 注册 + ClaudeAuthHandle state |
| `src/stores/claude-account-store.ts` | 登录流程 state/actions |
| `src/components/ToolsPanel/ProviderAuthDialog.tsx` | Claude Subscription 行真实化 |

## 不做的事

- 不做 `claude setup-token` 长期 token 路径（走订阅 OAuth 足够）
- 不自动 `fetchAuthStatus` on mount（dialog open 时才拉）
- 不处理 `--console` / `--sso` 模式（默认 `--claudeai` 即可）

## 验证

- `cargo test` — 通过（沿用已有 Codex OAuth helper 测试，Claude 无新增）
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：Dialog → Claude → Subscription radio → Login → 浏览器打开 → 登录成功后自动回填邮箱

## CM 回填区

- `23671c69` — `feat(claude): add subscription login/logout to Provider Authentication dialog` — 全链路实现
