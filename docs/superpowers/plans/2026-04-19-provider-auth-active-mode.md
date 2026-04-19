# Provider Authentication 改为严格 XOR active_mode

## 目标

- Dialog 从"Subscription 和 API key 并存 + 运行时优先"改成"每 provider 只选一种，Save 持久化 active 那份"
- Account 卡片同步展示当前 active 模式
- Dialog 高度固定，按钮粘底

## 背景

初版 `ProviderAuthDialog` 把 Subscription 行 + API key 输入垂直堆在一起，运行时 `apply_provider_auth` 有 key 就用、没 key 就回退订阅。用户反馈：
1. 没有 active 模式标识，看不出到底会生效哪一份
2. 两份数据并存容易误用（切到订阅后旧 key 还在存储里）
3. Dialog 高度随内容变化，底部按钮跟着飘
4. Save 后卡片没更新，仍展示订阅 profile

结论：**XOR 显式 active mode** 更清楚，并把 UI 同步到 active 状态。

## 实现

### 1. Schema v3 → v4

- `src-tauri/src/daemon/task_graph/persist.rs`
  - `SCHEMA_VERSION: 3 → 4`
  - `provider_auth` 表新增 `active_mode TEXT` 列（nullable）
  - `migrate_if_needed` 加 v3→v4 分支：`ALTER TABLE ... ADD COLUMN active_mode TEXT`
  - 新 helper `provider_auth_has_column` 做 PRAGMA 探测，幂等
- `ProviderAuthConfig` 增 `active_mode: Option<String>`，序列化 `skip_serializing_if = "Option::is_none"`
- persist INSERT / SELECT 补齐新列

### 2. Launch 层短路

- `src-tauri/src/daemon/codex/lifecycle.rs::apply_provider_auth`
- `src-tauri/src/daemon/claude_sdk/process.rs::apply_provider_auth`
- 新增条件：`if matches!(a.active_mode.as_deref(), Some("subscription")) { return }`
  - `active_mode == None` 保持老逻辑（按 api_key 有无推断，兼容 pre-v4 行）
  - `active_mode == "subscription"` → 强制不注入 env / `--config`，即使 api_key 仍存在
- 两侧各 +1 测试：subscription-mode short-circuit

### 3. UI

- `ProviderAuthDialog.tsx`：
  - `FormState` 新增 `activeMode: ActiveMode`
  - 每个 provider section 顶上加 `[Subscription | API Key]` 二选一按钮组（`ModeRadio`）
  - Subscription mode 渲染 `SubscriptionRow`（登录状态/按钮）
  - API key mode 渲染 key 输入 + Advanced
  - `toConfig`: subscription mode 保存时把 api_key/base_url 等都清空，避免旧值残留
- Dialog 壳：`h-[90vh] max-h-160 flex flex-col` → header + footer 固定，只中间滚动
- `AccountsInfoPanel`：
  - `AccountCard` 接 `mode?` prop，右上角画 `SUBSCRIPTION` / `API KEY` 徽章
  - 卡片订阅 `useProviderAuthStore.configs`，Save 后立即 re-render
  - API key mode 下卡片正文切换：Endpoint / Auth / Wire API / 末 4 位 key

### 4. Rust migration test

- `migration_v3_to_v4_adds_active_mode_column`：手写 v3 schema（含 v3 provider_auth 表）→ 打开 store → ALTER 执行 → 新列为 None

## 文件清单

| 文件 | 作用 |
|---|---|
| `src-tauri/src/daemon/task_graph/persist.rs` | v4 schema + migration |
| `src-tauri/src/daemon/task_graph/types.rs` | `active_mode` 字段 |
| `src-tauri/src/daemon/task_graph/tests.rs` | migration + fixture 更新 |
| `src-tauri/src/daemon/codex/lifecycle.rs` | subscription 短路 + 测试 |
| `src-tauri/src/daemon/claude_sdk/process.rs` | 同上 |
| `src-tauri/src/daemon/claude_sdk/process_tests.rs` | 补 subscription 测试 |
| `src/stores/provider-auth-store.ts` | `activeMode` 字段 + `ActiveMode` 类型导出 |
| `src/components/ToolsPanel/ProviderAuthDialog.tsx` | radio + 高度修复 + 保存语义 |
| `src/components/ToolsPanel/AccountsInfoPanel.tsx` | 卡片 mode 徽章 + 订阅 provider-auth-store |

## 不做的事

- Pre-v4 行（`active_mode == None`）保留自动推断以兼容旧数据；下次写入会显式填一个 mode
- 不加密凭据（文件 0600 + SQLite 本地，后续再升 keychain）
- 不做 per-agent mode（全局一份）

## 验证

- `cargo test` — 735 → 738 passed（+3 migration/subscription 测试）
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：Save subscription mode → 重启 → 卡片 subscription 徽章 + profile；Save api_key mode → 卡片 API KEY 徽章 + endpoint 摘要

## CM 回填区

- `84588008` — `fix(ui): fix ProviderAuthDialog height so footer sticks to the bottom` — `h-[90vh] max-h-160` 固定外壳，header+footer 不再飘
- `b2d591df` — `feat(provider-auth): replace "both on, key wins" with active_mode XOR` — schema v4 + launch subscription 短路 + dialog radio + 保存语义 + migration 测试
- `b5f1fb42` — `fix(tools): Account cards reflect active provider auth mode` — 卡片订阅 provider-auth-store + mode 徽章 + api-key 模式摘要视图
