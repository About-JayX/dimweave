# Report Telegram Design

## Summary

Dimweave currently sends Telegram notifications from a daemon-side hard-coded rule: terminal `lead -> user` messages are forwarded to the single configured Telegram chat. That rule is too rigid for the workflow the user wants. The next version should let the agent explicitly mark important lead messages for Telegram fan-out without adding a new routing/status enum.

The agreed protocol change is:

- keep `status` as the lifecycle field: `in_progress | done | error`
- add an optional boolean field: `report_telegram`
- keep Telegram configuration global
- keep a single configured Telegram `chat_id` for now
- include `task_id` in the Telegram message body instead of introducing task-specific Telegram config

## User-Confirmed Requirements

### Product behavior

Only messages explicitly marked with `report_telegram: true` should be considered for Telegram delivery.

The user confirmed these message classes should be sent to Telegram:

1. development plan drafting complete
2. development plan confirmation complete
3. each task review result
4. final review result
5. blocking errors only

The user also confirmed:

- Telegram config must stay global, not task-scoped
- a single Telegram chat target is enough for now
- `task_id` should be included in the Telegram message content
- Telegram output should look polished rather than plain text
- visual style preference: **B** ("slightly more ceremonial / card-like")

### Explicit non-goals

- no task-level Telegram routing config
- no multiple-chat fan-out in this version
- no expansion of `status` beyond `in_progress | done | error`
- no requirement that every lead terminal message reaches Telegram
- no Telegram delivery for non-blocking errors

## Current Architecture Facts

### Telegram routing is daemon-side and hard-coded

Today Telegram fan-out happens inside `route_message_with_display()` after normal message delivery succeeds. The current trigger ignores message intent and instead checks only:

- sender is `lead`
- recipient is `user`
- status is terminal

Then it forwards the message to the one configured paired chat.

Relevant files:

- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/telegram/types.rs`

### Agent protocols currently do not carry Telegram intent

Codex structured output only supports:

- `message`
- `send_to`
- `status`

Claude reply tool only supports:

- `to`
- `text`
- `status`

So even if the model wants Telegram fan-out, the signal is currently dropped.

Relevant files:

- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `bridge/src/tools.rs`
- `bridge/src/mcp_protocol.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`

### Telegram API still requires chat_id

Telegram delivery is ultimately `sendMessage(chat_id, text)`. `task_id` is not a routing destination and cannot replace the configured chat target. The right design is:

- global config decides **where** to send
- `task_id` in the message body explains **which task** the message belongs to

Relevant file:

- `src-tauri/src/telegram/api.rs`

## Design Goals

1. make Telegram delivery an explicit agent decision
2. keep routing/status semantics stable
3. keep token overhead minimal
4. preserve global Telegram configuration
5. produce readable, polished Telegram messages
6. avoid noisy Telegram spam from intermediate chatter

## Recommended Design

### 1. Add `report_telegram?: boolean` to message protocol

Add `report_telegram` as an optional boolean to the end-to-end message model:

- Rust daemon `BridgeMessage`
- bridge-side `BridgeMessage`
- frontend TypeScript `BridgeMessage`
- Codex structured output parser
- Codex output schema
- Claude `reply()` tool schema and parser

Default behavior:

- missing field == `false`

This keeps the protocol backward compatible for existing messages.

### 2. Keep `report_telegram` out of incoming agent context metadata

`report_telegram` is a daemon-side fan-out intent, not collaboration context. It should be preserved on the message object while routing, but it should **not** be added to Claude channel meta or re-injected into other agents' conversational text unless the agent explicitly authored it.

Reason:

- it reduces unnecessary token usage
- it avoids teaching downstream agents to cargo-cult the flag from incoming messages
- it keeps Telegram fan-out as a transport concern rather than a semantic status enum

### 3. Replace the Telegram trigger rule

Replace the current hard-coded rule with this gate:

- `msg.from == "lead"`
- `msg.report_telegram == true`
- `msg.status` is terminal (`done` or `error`)
- Telegram runtime is online
- Telegram notifications are enabled
- global configured `chat_id` exists

Important change:

- `msg.to` should **not** matter anymore

This allows lead to notify Telegram for plan handoffs, task review outcomes, and blocking errors even when the visible recipient is not `user`.

### 4. Keep blocking-error policy mostly prompt-driven, with light server safety

The user explicitly wants **blocking errors only**.

The primary enforcement should be prompt/protocol guidance:

- only mark `report_telegram=true` for blocking errors

Server-side guardrails should stay simple:

- `status=error` is eligible
- but non-blocking vs blocking remains an agent decision in this version

This is the smallest change set and avoids inventing a second classification field right now.

### 5. Use polished Telegram HTML formatting

Telegram should render a styled notification instead of plain text. Use Telegram `sendMessage` with `parse_mode = "HTML"` and format messages into a compact card-like block.

Recommended template shape:

- emoji + bold title line
- metadata block using `<b>` labels
- code formatting for ids/paths
- short summary body
- explicit blocker / next action lines

For example:

```html
<b>📋 Dimweave 任务审查结果</b>

<b>Task ID:</b> <code>task_123</code>
<b>Task:</b> Reply routing polish
<b>Status:</b> done
<b>Worktree:</b> <code>.worktrees/reply-polish</code>
<b>Blocker:</b> 无
<b>Next:</b> 进入下一个任务

<b>摘要</b>
已完成 task 2 审查，测试通过，记录 CM，等待最终 review。
```

Requirements for the formatter:

- escape all user/model text for HTML safety
- avoid giant paragraphs
- preserve chunking for Telegram's 4096-char limit
- keep layout consistent across all notification classes

### 6. Add prompt rules for predictable notification content

For `report_telegram=true` messages, lead should be instructed to produce concise, structured content that fits the Telegram template well.

Recommended prompt rule:

- the message should begin with a short event headline
- then include: what was done, what was verified, blocker/no blocker, next action

Recommended event headline labels:

- `计划编排`
- `计划确认`
- `任务审查`
- `最终审查`
- `阻塞错误`

This avoids introducing another protocol field while still making Telegram output readable and classifiable.

### 7. Preserve failure isolation

Telegram send failures must never block the main workflow. On delivery failure:

- log a warning/error
- do not retry inside the same route call
- do not change the original message routing result

## File-Level Impact

### Protocol / parsing

- `src-tauri/src/daemon/types.rs`
- `bridge/src/types.rs`
- `src/types.ts`
- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `bridge/src/tools.rs`
- `bridge/src/mcp_protocol.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`

### Telegram formatting / dispatch

- `src-tauri/src/telegram/api.rs`
- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/telegram/types.rs` (if state exposure needs to mention global chat readiness)

### Tests

- `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `bridge/src/tools_tests.rs`
- `src-tauri/src/telegram/report.rs` test module
- new Telegram API/body-format tests if formatter helpers are extracted

## Acceptance Criteria

1. lead can mark a message with `report_telegram=true` through both Codex structured output and Claude reply tool
2. Telegram fan-out no longer depends on `to == "user"`
3. Telegram fan-out still requires the global single chat target and runtime readiness
4. terminal task review / final review / plan notifications can reach Telegram
5. non-marked lead messages do not reach Telegram
6. Telegram messages render as HTML-formatted, readable notifications
7. send failures do not break primary routing

## Risks

### Existing local root-workspace diff

At the time this design was approved, the root workspace had an uncommitted local diff in `src-tauri/src/telegram/report.rs` that is not automatically present in the new worktree branch. Implementation must not overwrite or silently ignore that divergence during final integration.

### Prompt-only blocking-error classification

This design intentionally keeps "blocking vs non-blocking" as a prompt discipline rather than adding another protocol field. That keeps the version small, but it means noisy agent behavior is still possible if the prompt is weak.

## Recommendation

Proceed with the smallest protocol expansion that solves the user's routing need:

- add `report_telegram`
- keep global single chat config
- upgrade Telegram formatter to HTML
- keep `status` unchanged

This satisfies the workflow requirement with minimal token overhead and minimal disruption to the existing routing architecture.
