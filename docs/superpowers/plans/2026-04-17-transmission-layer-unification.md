# 2026-04-17 — Dimweave 传输层规范化

## Context

外部 PDF 报告 + 本轮多次审查把"lead→coder 通信失效"的根因收敛到：

1. **`TaskAgent` 是路由权威实体**（`src-tauri/src/daemon/task_graph/types.rs:111-121`），同 task 可多同-role agent。纯 role 字符串匹配在 shared-role 场景下会送错 lead。
2. **三条 wire 契约表面**（Claude MCP reply / Codex output_schema / `BridgeMessage` 存储）envelope 字段名（`text` / `message` / `content`）和 target 形态（松散 object / flat-required / discriminated enum）都分裂。
3. **Codex `build_completed_output_message` 有 fail-open 默认**：schema 启用但 `target` 缺失时走 `MessageTarget::User`，worker 结果绕过 lead 直达用户。
4. 诊断路径（silent turn / parse error / dropped / WS error）硬写 `MessageTarget::User`。

## 落地内容（一次性上线，5~6 个 commit 可切分）

### Step 1 — `MessageTarget` 自定义 serde
- 新增 `src-tauri/src/daemon/message_target.rs` + `bridge/src/message_target.rs`
- Rust `enum MessageTarget { User | Role { role } | Agent { agent_id } }` 变体保留，手写 `Serialize`/`Deserialize` 发射扁平 3 字段 `{kind, role, agentId}`（未用字段空串）
- Deserialize 兼容老判别联合形态（task_graph 持久化数据不破）
- 18 个回归测试（daemon 侧）+ 11 个（bridge 侧）

### Step 2 — envelope `text`/`content` → `message`
- `BridgeMessage.content` → `message`（daemon + bridge Rust 字段名一并改）
- Bridge MCP reply tool input: `text` → `message`
- `daemon_send_user_input` Tauri 参数 + `DaemonCmd::SendUserInput` 字段同步
- 前端 TS `BridgeMessage.content` / `sendToCodex(content, ...)` / fixture JSON 一并改
- `MessageMarkdown` 组件 prop 名保留 `content`（通用 markdown 渲染 API，不是 wire 字段）

### Step 3 — bridge schema 扁平化对齐 Codex
- `reply_tool_schema::target` 改为 `required: [kind, role, agentId]` + `additionalProperties: false`，与 `role_config::output_schema` 完全一致

### Step 4 — Codex output_schema 已 canonical，无需改

### Step 4b — Codex fail-closed target path
- `build_completed_output_message` 返回类型改为 `CompletedOutput { Ready | Skip | MissingTarget }`
- `MissingTarget` 分支走 `worker_diagnostic_target` 回到 delegating lead，不再默认 `User`

### Step 4c — Routing sender-role soft guard
- `route_message_inner_with_meta` 在 `is_to_user()` 分支前 log WARN 当 source_role ∈ {coder, reviewer}
- 不硬拒（LLM 偶尔合法），观测模式

### Step 5 — worker 诊断 helper 按 agent_id
- `routing.rs` 新增 `pub fn delegator_agent_id(sender_agent_id) -> Option<String>`
- `session_event.rs::worker_diagnostic_target(state, sender_role, sender_agent_id, task_id)`：P1 reply_target_map → P2 first lead in `agents_for_task` (WARN) → P3 User
- 替换 4 处硬写 `MessageTarget::User` 的诊断路径
- `build_silent_turn_fallback` 签名加 `target: MessageTarget` 参数

### Step 6 — 诊断 helper 测试
- `worker_diagnostic_target_keeps_lead_sender_at_user`
- `worker_diagnostic_target_falls_back_to_first_lead_in_task`
- `worker_diagnostic_target_returns_user_when_no_lead_known`
- `silent_turn_fallback_uses_provided_target_verbatim`
- `completed_output_builder_fails_closed_when_target_missing`

### Step 6b — Parser 对 legacy `to` 精确报错
- `parse_target` 检测 `args["to"] && !args["target"]` 时返回 `"legacy 'to' field detected..."` 错误

### Step 7 — `<channel>` 元数据双向透传
- Claude 侧：`wrap_channel_content(from, content, sender_agent_id, task_id)` 新增两个可选属性
- Codex 侧：`build_codex_text` 给非 user 消息加 `[agent_id]` 和 `(task: tid)` 标记
- 2 个新测试锁定 attr 透传行为

### Step 8 — Prompt 教学 agent_id-first
- `claude_prompt.rs`：Communication 段 + Routing Examples 段加"按 `sender_agent_id` 回复 delegator"教学
- `roles.rs` (Codex prompt)：同样加 agent_id-first routing 引导
- 新测试 `prompt_teaches_agent_id_targeting_via_sender_agent_id`

### Step 9 — CI drift guard
- `scripts/check_contract_drift.sh`：grep 检测 `reply(to=...)`、`reply(target, text, status)`、`reply(to="...")` 在活跃文档中的残留
- 仅检查 `.claude/agents`、`.claude/rules`、`.claude/skills`、`docs/agents`（排除历史 plan 文档）

### Step 10 — 文档与 chain 记录
- `.claude/agents/{nexus-lead,nexus-coder,nexus-reviewer}.md`：重写，加 reference-only 提示 + 所有示例用新 canonical 形态
- `.claude/rules/daemon.md`：reply 契约行改为 `target + message + status`
- `docs/agents/claude-message-delivery.md`：老 `reply(to=...)` 示例全改
- `docs/agents/codex-chain.md` / `docs/agents/claude-chain.md`：追加本次修复记录

## 验证快照

| 层 | 结果 |
|---|---|
| `cargo test -p dimweave-bridge` | **53/53 passed** |
| `cargo test -p dimweave` | **691 passed, 3 failed**（全部 pre-existing：2 state_persistence 文件权限 + 1 `reply_target_map` 静态污染 flake） |
| `scripts/check_contract_drift.sh` | **OK** |
| `bun x tsc --noEmit` | 18 pre-existing errors（bun:test 类型缺失等），我的改动零新错 |

## Canonical wire contract（本次落地后）

```json
{
  "target": {
    "kind": "user" | "role" | "agent",
    "role": "<role name or ''>",
    "agentId": "<agent id or ''>"
  },
  "message": "<the response body>",
  "status": "in_progress" | "done" | "error"
}
```

**三处全一致**：Claude MCP `reply` input、Codex `output_schema`、`BridgeMessage` wire 序列化。

## 已确认设计决策

- Envelope 字段统一为 `message`（不是 `text` 也不是 `content`）
- Target shape 扁平 + 3 字段全必填（OpenAI strict schema 唯一兼容形态）
- Rust `MessageTarget` 变体保留（类型安全），自定义 serde 决定 wire 形态
- 持久化后向兼容：老判别联合 JSON 仍能反序列化
- `MessageSource` 不改（daemon 单向写入，不由模型生成）
- 多-lead 同 task 兜底：reply_target_map → first-lead-by-order（WARN）→ User

## 明确不做

- ❌ `text` 子协议（JSON-in-JSON，对 LLM 输出太脆弱）
- ❌ 任务状态机 schema（绑在子协议上，去掉后无载体）
- ❌ feature flag `DIMWEAVE_REPLY_COMPAT_MODE`（原子上线，不需灰度）
- ❌ `.claude/agents/*` + `claude_prompt.rs` 从同模板 codegen（4 个文件 + CI drift grep 已够）

## Supersedes

- `docs/superpowers/plans/2026-04-17-lead-coder-communication-fix.md`（初版局部修，本版扩展为全 wire 契约规范化）

## CM (Configuration Management)

### Commit 1 — pre-session UI + version bump
- **Hash**: `9c098e6f`
- **Subject**: `chore: stabilize MessageList scroller + bump to DimweaveV3 0.3.0`
- **Scope**: unrelated to the plan, kept intact per user request.
  - `src/components/MessagePanel/MessageList.tsx` — freeze Virtuoso `scrollerRef` callback identity via `useCallback` with empty deps to break a null → el oscillation loop.
  - `src-tauri/tauri.conf.json` — product name `DimweaveV3`, version `0.3.0`.

### Commit 2 — atomic transmission layer unification
- **Hash**: `5eeabae3`
- **Subject**: `feat(daemon): unify transmission layer on agent_id-aware flat target + message envelope`
- **Files**: 60 modified, 5 added (see commit body for inventory).
- **Scope**: executes every step (1 through 10) + late-stage reviewer/runtime cleanups in a single atomic change. Details:
  - Step 1: `MessageTarget` custom serde (daemon + bridge mirror) + 29 new regression tests (18 daemon + 11 bridge).
  - Step 2: envelope `text`/`content` → `message` across daemon, bridge, frontend, Tauri command (`daemon_send_user_input` + `DaemonCmd::SendUserInput`).
  - Step 3: bridge `reply_tool_schema` flat with all 3 target fields required + `additionalProperties:false`.
  - Step 4b: `build_completed_output_message` returns `CompletedOutput { Ready | Skip | MissingTarget }`; `MissingTarget` routes diagnostic back to lead instead of defaulting to user.
  - Step 4c: `route_message_inner_with_meta` emits soft-guard WARN log when non-lead role targets user.
  - Step 5: `routing::delegator_agent_id()` pub accessor; `worker_diagnostic_target(state, sender_role, sender_agent_id, task_id)` resolves via `reply_target_map` → first lead in `agents_for_task` → user.
  - Step 6: 4 new diagnostic-helper tests; `silent_turn_fallback` accepts pre-computed target param.
  - Step 6b: `parse_target` detects legacy `{to:...}` args and returns precise self-correction hint.
  - Step 7: `wrap_channel_content(from, content, sender_agent_id, task_id)` adds optional `<channel>` attrs; `build_codex_input_items` keeps `[agent_id]` label on non-user messages. `[task: ...]` user-input prefix was tested and rolled back (see runtime notes).
  - Step 8: `claude_prompt.rs` + `roles.rs` teach agent_id-first precision reply; `role_protocol::role_specific_rules("lead")` gains `## Lead Escalation Gate (ABSOLUTE)` block restricting `target=user` to 4 gate scenarios (plan approval / external blocker / final acceptance / blocked stage complete) and explicitly banning mid-execution `in_progress → user` / ack-to-user / coder-progress-forwarding / multi-output-per-turn.
  - Step 9: `scripts/check_contract_drift.sh` — grep-based CI drift guard on active docs only.
  - Step 10: chain records (`docs/agents/{claude,codex}-chain.md`) + agent reference docs + rules doc + legacy `reply(to=...)` references in `docs/agents/claude-message-delivery.md`.
- **Late-stage cleanups captured in this commit**:
  - Reviewer findings: `control/handler.rs:117` stale `content` → `message` log string; `state_persistence_tests.rs` `pragma_conn` no-op removed with explanatory comment; `#[serde(alias = "content")]` on `BridgeMessage.message` both crates to preserve legacy persisted JSON in `buffered_messages`.
  - Runtime observation: `build_codex_text` no longer injects `[task: <uuid>]` prefix for user messages (Codex model treated it as directive and emit'd multiple agentMessages per turn). `(task: ...)` suffix on agent-source messages also removed as redundant (Codex sessions are already task-bound).
  - Runtime observation: `Lead Escalation Gate (ABSOLUTE)` prompt block added after the first dev-session test showed lead looping `target=user, status=in_progress` 11 times per user input despite the existing "Autonomous Execution Mode" rule being buried in the prompt tail.
  - Test hygiene: `clear_reply_targets()` at entry of `mixed_provider_guard_coder_to_lead_delivered` (process-wide static was leaking entries from sibling tests).
  - Test rewrites for SQLite semantics: `state_persistence_tests` `create_task_does_not_auto_persist` now checks `tasks` row count via read-only connection; `save_task_graph_returns_err_on_unwritable_path` now exercises `SQLITE_BUSY` via `BEGIN IMMEDIATE` on a contender connection (fs-permission approach unreliable under macOS SIP + SQLite WAL).
- **Verification**:
  - `cargo test -p dimweave-bridge` — 53/53 pass
  - `cargo test -p dimweave` — 698/698 pass (3 previously pre-existing failures now resolved)
  - `scripts/check_contract_drift.sh` — OK
  - `bun x tsc --noEmit` — no new errors introduced (18 pre-existing bun:test/ImportMeta errors unchanged)
  - Runtime: `bun run tauri dev` boot clean, Codex app-server + Vite up; manual E2E of the lead escalation gate pending user session restart to pick up new `base_instructions` at next `thread/start`.

### Commit 3 — CM backfill (this file)
- **Hash**: (will be filled after commit)
- **Subject**: `docs: record CM for transmission layer unification plan`
- **Scope**: this file only.

## Post-Release Addendum

### Known runtime behavior to verify after lead-session restart
The prompt changes take effect only when the Codex `thread/start` runs
with the new `base_instructions`, i.e. the user must restart the Codex
lead session in the UI. Expected behavior on the first user turn post-restart:

1. Lead's first output is `target={"kind":"user"}, status="in_progress"` carrying a **plan proposal** (not an ack, not a delegation).
2. On user confirmation, lead emits `target={"kind":"role","role":"coder",...}, status="in_progress"` with the decomposed task.
3. Between plan approval and final acceptance, lead must NOT emit `target=user` — user visibility comes from the GUI observing the lead↔coder transcript.
4. On blocker, lead emits `target={"kind":"user"}, status="error"` with the specific dependency requested.
5. On final completion, lead emits `target={"kind":"user"}, status="done"` summary.

If lead still emits multiple `target=user, status=in_progress` messages during execution, the prompt soft-lock has failed and a daemon-level hard lock is required (see `docs/superpowers/plans/2026-04-17-transmission-layer-unification.md` Scope section — daemon-level "Option C" was deliberately deferred to observe prompt compliance first).
