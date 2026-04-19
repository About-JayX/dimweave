# Provider Authentication Dialog —— 第三方 endpoint + 统一登录入口

## 目标

一个齿轮图标（复用 `Settings2` + Task edit icon 样式），点开后是统一 dialog，覆盖两 provider 的：
- Subscription 登录（OAuth）
- API key 登录（官网或第三方 endpoint）

启动 Claude / Codex 子进程时按配置注入 env / `--config` 实现第三方 endpoint 支持。

## 确认项（已对齐）

| 议题 | 结论 |
|---|---|
| 齿轮位置 | Accounts section header 右上（和 Telegram/飞书 同层级） |
| Subscription 流程 | 搬进 dialog，卡片只展示状态 |
| 两种认证 | 并存，运行时 API key 优先 |
| 生效时机 | 下次 launch；已跑的会话不动 |
| Config 范围 | 全局（不按 task/workspace） |
| 凭据加密 | 暂不加密，SQLite + 文件权限 0600，后续升 keychain |

## 底层 CLI 支持（已验证）

**Claude Code**
- env `ANTHROPIC_BASE_URL` — override endpoint
- env `ANTHROPIC_API_KEY` — x-api-key header 形式
- env `ANTHROPIC_AUTH_TOKEN` — Bearer 形式

**Codex** — `--config KEY=VALUE` 已在 `lifecycle.rs::start` 使用
- `--config model_provider="<name>"`
- `--config model_providers.<name>.base_url="..."`
- `--config model_providers.<name>.env_key="..."`
- `--config model_providers.<name>.wire_api="chat|responses"`
- 内置 provider id 不可覆盖：`openai / chatgpt / codex / atlas`，自定义 name 必须外加前缀/后缀

## 数据层

### Schema（task_graph.db v2 → v3）

```sql
CREATE TABLE provider_auth (
  provider     TEXT PRIMARY KEY,   -- "claude" | "codex"
  api_key      TEXT,               -- 明文，0600
  base_url     TEXT,               -- NULL = 官网
  wire_api     TEXT,               -- codex only: "chat" | "responses"
  auth_mode    TEXT,               -- claude only: "api_key" | "bearer"（默认 bearer）
  provider_name TEXT,              -- codex only: "dimweave-openrouter" 等
  updated_at   INTEGER NOT NULL
);
```

- Migration 幂等：检查 meta.schema_version，<3 时 CREATE TABLE
- Subscription 凭据**不存这个表**，仍由 CLI / keychain 管
- 两行固定（claude / codex），CRUD 语义为 upsert

### Rust 类型

```rust
pub struct ProviderAuthConfig {
    pub provider: String,  // "claude" | "codex"
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub wire_api: Option<String>,
    pub auth_mode: Option<String>,
    pub provider_name: Option<String>,
    pub updated_at: u64,
}
```

## 实施步骤

### Step 1 — task_graph schema v3
- `persist.rs::SCHEMA_VERSION: 2 → 3`
- CREATE TABLE + `migrate_if_needed` 加 v2→v3 分支
- Store 新增 `get_provider_auth(provider)` / `upsert_provider_auth(cfg)` / `delete_provider_auth(provider)`
- Round-trip + migration 测试

### Step 2 — Tauri commands
- `commands_provider_auth.rs`（新文件）
  - `daemon_get_provider_auth(provider: String) -> Option<ProviderAuthConfig>`
  - `daemon_save_provider_auth(config: ProviderAuthConfig) -> ()`
  - `daemon_clear_provider_auth(provider: String) -> ()`
- 注册到 `main.rs::invoke_handler`

### Step 3 — Launch 运行时集成

**Codex** (`daemon/codex/lifecycle.rs::start()`):
```rust
pub async fn start(
    port: u16, codex_home: &Path, cwd: &str,
    sandbox_mode: &str, approval_policy: &str,
    auth: Option<&ProviderAuthConfig>,  // NEW
) -> anyhow::Result<Child> {
    let mut cmd = Command::new(...);
    cmd.arg("app-server").arg("--listen").arg(...)
       .arg("--config").arg("sandbox_mode=...")
       .arg("--config").arg("approval_policy=...")
       .arg("--config").arg("features.apply_patch_freeform=false");

    if let Some(a) = auth {
        if let (Some(key), Some(url)) = (&a.api_key, &a.base_url) {
            // 第三方 endpoint
            let pname = a.provider_name.clone()
                .unwrap_or_else(|| "dimweave-custom".into());
            let env_var = format!("DIMWEAVE_{}_KEY", pname.to_uppercase().replace('-', "_"));
            let wire = a.wire_api.as_deref().unwrap_or("chat");
            cmd.arg("--config").arg(format!("model_provider=\"{pname}\""))
               .arg("--config").arg(format!("model_providers.{pname}.base_url=\"{url}\""))
               .arg("--config").arg(format!("model_providers.{pname}.env_key=\"{env_var}\""))
               .arg("--config").arg(format!("model_providers.{pname}.wire_api=\"{wire}\""));
            cmd.env(env_var, key);
        } else if let Some(key) = &a.api_key {
            // 官网 + API key：直接塞 OPENAI_API_KEY
            cmd.env("OPENAI_API_KEY", key);
        }
    }
    cmd.env("CODEX_HOME", codex_home).env("PATH", &path)...
}
```

**Claude** (`daemon/claude_sdk/*` spawn 处):
```rust
if let Some(a) = auth {
    if let Some(key) = &a.api_key {
        let mode = a.auth_mode.as_deref().unwrap_or("bearer");
        match mode {
            "api_key" => { cmd.env("ANTHROPIC_API_KEY", key); }
            _ => { cmd.env("ANTHROPIC_AUTH_TOKEN", key); }
        }
        if let Some(url) = &a.base_url {
            cmd.env("ANTHROPIC_BASE_URL", url);
        }
    }
}
```

**调用点**：
- `daemon/codex/mod.rs::start_session` — 在 `resolve_role_launch_config` 之后 `state.task_graph.get_provider_auth("codex")` 拿 config，传下去
- Claude SDK launch — 类似

### Step 4 — TS store `useProviderAuthStore`
- `src/stores/provider-auth-store.ts`
- `configs: { claude?: ProviderAuthConfig, codex?: ProviderAuthConfig }`
- `fetchAll()` / `save(cfg)` / `clear(provider)`

### Step 5 — UI `ProviderAuthDialog`
- `src/components/ToolsPanel/ProviderAuthDialog.tsx`（新）
  - Props: `open`, `onOpenChange`
  - 两栏（Claude / Codex）：
    - Subscription 行：如果已登录显示邮箱 + Logout；否则显示对应 Login 按钮（复用已有 store action `codexAccountStore.login` / Claude OAuth 未来补）
    - API Key 输入（password 字段）
    - Advanced 折叠：base_url / wire_api / auth_mode / provider_name
  - Save：`saveProviderAuth({ provider: "claude", ... })` + `saveProviderAuth({ provider: "codex", ... })`
- `AccountsInfoPanel`（或 ToolsPanel）头部加 `Settings2` 按钮打开 dialog
- **精简 CodexCard / ClaudeCard**：把 AuthActions 从卡片里移除（搬进 dialog），卡片只保留状态展示 + 用量 + refresh

### Step 6 — 齿轮放在 Accounts section header
- 修改 `ToolsPanel/index.tsx` 的 `DisclosureSection` 接 trailing action（或直接在 `AccountsInfoPanel` 顶部加按钮）
- 样式复用 TaskHeader 的 edit icon：`Settings2 size-3.5`、`rounded-lg p-1.5 text-muted-foreground/50 hover:bg-muted hover:text-foreground`

### Step 7 — 测试
- Rust: `persist_provider_auth_round_trip` / `upsert_updates_existing_row` / `migration_v2_to_v3_creates_provider_auth`
- Rust: `start_codex_adds_provider_config_when_auth_has_base_url`（可参数化 mock 测）
- TS: store 基本 action 测

### Step 8 — 文档
- CLAUDE.md / `.claude/rules/frontend.md` / `tauri.md` 新 command 列表
- 本 plan CM 回填

## 不做的事

- TaskAgent 级 provider binding（粒度粗，全部 agent 共享全局配置）
- Keychain 加密（后续升级）
- 多 profile 切换（MVP 只一个覆盖；要多套用 env var 文件或 shell 切）
- 影响已运行的 agent（已跑的会话不动）

## 验证

- `cargo test`
- `bun x tsc --noEmit -p tsconfig.app.json`
- `bun run build`
- 手工：
  1. Subscription 模式，空 API key → 默认流程（和现在一样）
  2. API key 有值、base_url 空 → `.env(OPENAI_API_KEY=...)` / `.env(ANTHROPIC_AUTH_TOKEN=...)` 官网走 key 鉴权
  3. API key + base_url（如 OpenRouter）→ Codex `--config model_providers.dimweave-xxx.*`；Claude `ANTHROPIC_BASE_URL`
  4. Dialog 关闭不保存不影响现有 auth.json

## Commit 规划

1. `refactor(task_graph): add provider_auth table with v2→v3 migration`
2. `feat(provider-auth): Tauri CRUD commands for third-party endpoint config`
3. `feat(launch): inject provider auth env/--config at Codex + Claude spawn`
4. `feat(ui): Provider Authentication dialog unifies subscription + API key paths`

## CM 回填区

- `1d9428ef` — `refactor(task_graph): add provider_auth table with v2→v3 migration` — 新 `provider_auth` 表 + `ProviderAuthConfig` 类型 + store `get/upsert/clear` + round-trip 和 v2→v3 迁移测试
- `d7091abd` — `feat(provider-auth): Tauri CRUD commands for third-party endpoint config` — 3 个 DaemonCmd 变体 + `commands_provider_auth` 包装，save 前校验 provider ∈ {claude, codex}
- `551e93ac` — `feat(launch): inject provider auth env/--config at Codex + Claude spawn` — `apply_provider_auth` helpers（Codex: `--config model_providers.*` + `DIMWEAVE_<NAME>_KEY` env / 或 `OPENAI_API_KEY`；Claude: `ANTHROPIC_AUTH_TOKEN|API_KEY` + 可选 `ANTHROPIC_BASE_URL`）+ 16 条单测
- `c7b585cb` — `feat(ui): Provider Authentication dialog unifies subscription + API key paths` — `useProviderAuthStore` + `ProviderAuthDialog`；`AccountsInfoPanel` 头部加齿轮入口，卡片移除旧 `AuthActions`

### 验证
- `cargo test` — 735 passed（+16 新 Codex/Claude apply 单测、+4 provider_auth CRUD/migration、+1 v2→v3 migration）
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
