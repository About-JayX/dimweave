# Claude 模型列表动态化（像 Codex 一样请求 API）

## 目标

删除 `CLAUDE_MODEL_OPTIONS` 硬编码，改为从 `GET /v1/models` 拉取，支持 Opus 4.7 等新模型随上游发布自动可见，同时按模型 capabilities 动态过滤 Effort 选项。

## 根因

`src/components/ClaudePanel/ClaudeConfigRows.tsx` 的 `CLAUDE_MODEL_OPTIONS` 与 `CLAUDE_EFFORT_OPTIONS` 写死，每次上游发新模型（Opus 4.7、xhigh effort 等）都要手工更新。Codex 侧已经用 `~/.codex/models_cache.json` + store 做到动态；Claude 侧还是 hardcoded。

## 可行性确认（已验证）

`security find-generic-password -s "Claude Code-credentials" -w` 读出 Claude Code keychain 里的 OAuth access token，配合 `anthropic-beta: oauth-2025-04-20` header 调 `GET https://api.anthropic.com/v1/models?limit=20` 返回全量列表，字段含 `id / display_name / capabilities.effort.{low,medium,high,max} / capabilities.thinking.types.adaptive`。

## Canonical shape

```rust
pub struct ClaudeModel {
    pub slug: String,
    pub display_name: String,
    pub supported_efforts: Vec<String>,  // e.g. ["low","medium","high","max"]
}
```

TS 镜像：

```ts
interface ClaudeModelInfo {
  slug: string;
  displayName: string;
  supportedEfforts: string[];
}
```

## 实施步骤

### Step 1 — Rust 模块 `src-tauri/src/claude/models.rs`
- `ClaudeModel` 结构 + `#[derive(Serialize)]` camelCase
- `list_models() -> Result<Vec<ClaudeModel>, String>`
  - 读 OAuth token：macOS 调 `security find-generic-password -s "Claude Code-credentials" -w`，parse `{"claudeAiOauth":{"accessToken":"..."}}`
  - 其他平台：读 `$ANTHROPIC_API_KEY` env 兜底
  - HTTP GET `https://api.anthropic.com/v1/models?limit=50` with `Authorization: Bearer <token>` + `anthropic-version: 2023-06-01` + `anthropic-beta: oauth-2025-04-20`
  - 用 `reqwest` 或现有 HTTP client；`serde_json::from_str` 解析
  - 从 `capabilities.effort.{low,medium,high,max}.supported == true` 收集 `supported_efforts`
  - 过滤掉 `claude-2*` / `claude-3*` 等老旧 id（只保留 4.x+）
- `claude/mod.rs` 增 `pub mod models;`

### Step 2 — Tauri command `list_claude_models`
- `src-tauri/src/main.rs`：注册到 `invoke_handler`
- `src-tauri/src/commands.rs` 或新文件：async 包装 `claude::models::list_models`，失败返回 `Err(String)` 让前端兜底

### Step 3 — TS store `claude-account-store`
- `src/stores/claude-account-store.ts`（镜像 `codex-account-store.ts`）
- `ClaudeModelInfo` 类型、`models: ClaudeModelInfo[]`、`fetchModels()` action
- 失败时 `models` 设为空数组；前端代码对空数组回退到 fallback 列表

### Step 4 — Fallback 常量
- `ClaudeConfigRows.tsx` 保留现有 `CLAUDE_MODEL_OPTIONS` / `CLAUDE_EFFORT_OPTIONS`，但去掉本次新加的 `claude-opus-4-7` / `xhigh` 硬编码（回滚到 plan 前状态）
- 改名注释为 `FALLBACK_CLAUDE_MODEL_OPTIONS`（语义清晰），仅当 API 失败时使用

### Step 5 — TaskSetupDialog 消费
- `src/components/TaskPanel/TaskSetupDialog.tsx`
  - Props 新增 `claudeModels?: ClaudeModelInfo[]`
  - `AgentConfigForm` 里：
    ```ts
    const isClaude = def.provider === "claude";
    const mOpts = isCodex && codexModels?.length
      ? codexModels.map(...)
      : isClaude && claudeModels?.length
      ? claudeModels.map((m) => ({ value: m.slug, label: m.displayName }))
      : caps.modelOptions;
    ```
  - Effort：claude 选中模型后按 `supportedEfforts` 过滤
- `src/components/TaskPanel/index.tsx`：从 `useClaudeAccountStore` 读 models + fetchModels，`useEffect` 拉取，传入 dialog

### Step 6 — ClaudePanel 消费
- `src/components/ClaudePanel/index.tsx` 或 `ClaudeConfigRows.tsx` 父组件：同样从 store 读，传入动态 options；失败兜底 FALLBACK

### Step 7 — 测试
- `src-tauri/src/claude/models_tests.rs`
  - `parses_models_response_extracts_supported_efforts`
  - `filters_out_legacy_claude_2_3_models`
  - `returns_err_when_token_missing`
- 前端 `tsc` + `bun run build` 通过

### Step 8 — 文档
- CLAUDE.md `.claude/rules/frontend.md` 新事件/命令：`list_claude_models`
- 本 plan 的 CM 回填区

## 不做的事

- **不做 token 刷新**：access_token 过期时 API 返回 401，store 捕获后 `models = []`，前端自动用 fallback；依赖 Claude CLI 自己维护 token
- **不缓存到磁盘**：Codex 用磁盘缓存是因为 Codex CLI 自己维护；Claude 侧直接每次进程启动时拉一次 + 在 store 里内存缓存
- **不支持 Bedrock/Vertex**：这些环境不走 Anthropic API，OAuth 读不到 token；fallback 生效

## 验证

- `cargo test -p dimweave` — 含 models_tests
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
- 手工：
  1. app 启动 → Claude 模型下拉应出现 Opus 4.7（已在 API 响应里）
  2. 切 Opus 4.7 → Effort 下拉只显示 API 报告的 `low/medium/high/max`
  3. 断网 / 无 keychain（非 macOS）→ 下拉回落到 FALLBACK 列表

## Commit 规划

1. `feat(claude): list models via Anthropic API with keychain OAuth`
2. `feat(ui): dynamic Claude model/effort options from API`
3. `docs: record CM for dynamic Claude models plan`

## CM 回填区

- `4b4096e7` — `feat(claude): list models via Anthropic /v1/models with keychain OAuth` — 新增 `src-tauri/src/claude/models.rs`（macOS keychain OAuth + `ANTHROPIC_API_KEY` 兜底 + `oauth-2025-04-20` beta header + 过滤 legacy claude-2/3 + 解析 `capabilities.effort`），注册 `list_claude_models` Tauri command，7 条 parser/filter 单测
- `eb3d941f` — `feat(ui): dynamic Claude model/effort options from Anthropic API` — 新增 `claude-account-store`，`TaskSetupDialog` Props 增 `claudeModels` 并在 `AgentConfigForm` 里按 `supportedEfforts` 过滤 effort；`ClaudeConfigRows` mount 时自动拉取，失败回退到 `CLAUDE_MODEL_OPTIONS` / `CLAUDE_EFFORT_OPTIONS`

### 验证
- `cargo test` — 711 passed（+7 新增）
- `bun x tsc --noEmit -p tsconfig.app.json` — 无新增错误
- `bun run build` — OK
