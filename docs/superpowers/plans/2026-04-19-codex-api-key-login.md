# Codex API Key 登录（与订阅登录并存）

## 目标

保留现有 `codex login`（ChatGPT OAuth 订阅）路径，新增 `codex login --with-api-key` 直接喂入 API key 的路径，两种登录从 UI 都可直达。

## 背景

- 已有 `src-tauri/src/codex/oauth.rs::start_login` 走 OAuth，把凭据写入 `~/.codex/auth.json`
- Codex CLI 支持 `codex login --with-api-key`（stdin 读 key）——把 OpenAI key 同样存到 `auth.json`
- 当时还没做 `Provider Authentication dialog`；API key 入口必须先挂在 `AuthActions` 里

## 实现

### 1. Rust: `codex::oauth::login_with_api_key`

- `src-tauri/src/codex/oauth.rs` 新增 `pub async fn login_with_api_key(api_key: String)`
- spawn `codex login --with-api-key` → 通过 `child.stdin.write_all(key.as_bytes())` 喂入 → 关闭 stdin → 等 exit
- 错误包装 stderr/stdout 便于 UI 展示

### 2. Tauri command

- `commands/oauth.rs::codex_login_with_api_key` 调用 Rust 层
- 注册到 `main.rs::invoke_handler`

### 3. TS store

- `useCodexAccountStore` 增加 `apiKeyLoginPending` / `apiKeyLoginError` 状态
- 新 action `loginWithApiKey(apiKey)`：invoke → success 自动 `fetchProfile` + `fetchModels` → 更新 state

### 4. UI

- `AuthActions.tsx`：
  - 未登录：保留 `Login to Codex (ChatGPT)` 主按钮，底下小字 `Use API key instead` 进入 `ApiKeyForm`
  - 已登录：邮箱行下方加小字 `Switch to API key`，允许直接覆盖现有 auth.json 而不用先 Logout
- `ApiKeyForm`：password input（`sk-...`）+ Submit / Cancel；成功后关闭表单并刷新 profile

## 文件清单

| 文件 | 改动 |
|---|---|
| `src-tauri/src/codex/oauth.rs` | `login_with_api_key` 函数 |
| `src-tauri/src/commands/oauth.rs` | `codex_login_with_api_key` command |
| `src-tauri/src/main.rs` | 注册 command |
| `src/stores/codex-account-store.ts` | `loginWithApiKey` action + state |
| `src/components/AgentStatus/AuthActions.tsx` | ApiKeyForm + 两个切换入口 |

## 不做的事

- 不校验 key 格式（由 `codex login --with-api-key` 自己验）
- 不做凭据加密（沿用 `~/.codex/auth.json` 的 0600 文件权限）
- 不覆盖 base_url（OpenAI 官网；第三方 endpoint 由后续 `Provider Authentication dialog` 处理）

## 验证

- `cargo test codex::` — 通过
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：未登录 → 点 API key 输入 → 提交 → 邮箱出现；已登录 → 点 Switch → 新 key 覆盖

## CM 回填区

- `68836125` — `feat(codex): add API key login alongside subscription OAuth` — 新增 Rust/TS 路径 + 未登录态的 `Use API key instead` 入口 + `ApiKeyForm`
- `52239d68` — `feat(codex): expose API-key login switch when already authenticated` — 已登录态增加 `Switch to API key` 切换入口
