# Dimweave Deep Performance And UX Rebuild Plan

> **For agentic workers:** REQUIRED SUB-SKILLS: Use `superpowers:subagent-driven-development` (preferred) or `superpowers:executing-plans` to implement this plan checkpoint-by-checkpoint. Every checkpoint must also run the `superpowers:requesting-code-review`, `superpowers:receiving-code-review`, and `superpowers:verification-before-completion` workflow before commit. Steps use checkbox syntax for tracking.

**Goal:** Rebuild Dimweave so it stays fluid under long-running Claude/Codex sessions, reduces UI noise, and behaves more like a professional agent IDE than an internal diagnostics surface.

**Product Direction:** Keep the balanced optimization strategy and aggressive UI restructure, but execute it in two stages: first remove the measured hot-path bottlenecks, then reshape the shell around a calmer professional IDE flow.

**Architecture:** Keep the existing Claude `--sdk-url` path, Codex app-server path, task/session model, and bridge-assisted MCP runtime. Do not assume the bridge disappears. First optimize the current hot transport and render paths in place; only introduce broader runtime transport abstraction if post-fix profiling still shows the Tauri event path as a real bottleneck.

**Tech Stack:** Tauri 2, Rust/Tokio daemon, React 19, Zustand, Tailwind 4, Bun, Cargo

---

## Current Facts

- The app shell subscribes to broad bridge store slices, so incoming message activity can rerender the entire layout.
- The message path mixes persistent timeline rows with transient stream indicators, and the current Virtuoso `totalCount` changes when stream indicators appear or disappear.
- `react-markdown` sits on the hot render path with no memoization, no plain-text fast path, and no lazy-loading.
- Logs are still rendered by full `.map()` instead of virtualization.
- Claude stream preview text is emitted one delta at a time in Rust, while the frontend currently drops the preview update entirely in `handleClaudeStreamEvent("preview")`.
- Codex and Claude stream indicators subscribe too broadly and do extra slicing/derived rendering on each update.
- The main bundle currently builds as a single JS chunk around `507.49 kB` minified, with no manual chunking strategy in `vite.config.ts`.
- The UI still gives too much equal-weight visual priority to task context, provider controls, logs, approvals, stream state, and final chat output.
- The visual system still overuses glow, blur, shadow, and `transition-all`.

---

## Checkpoint Delivery Contract

- Each checkpoint below must land as one focused commit. Do not batch multiple checkpoints into one review cycle.
- Each checkpoint must start with the relevant implementation workflow:
  - use `superpowers:test-driven-development` for behavior changes or regressions
  - use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to carry out the checkpoint
- Each checkpoint must end with the same review loop:
  1. run the checkpoint's targeted verification
  2. dispatch `superpowers:code-reviewer` via `superpowers:requesting-code-review`
  3. evaluate every finding with `superpowers:receiving-code-review`
  4. fix blocking and important findings, rerun the targeted verification, and re-request review until the checkpoint is clear
  5. run fresh completion evidence with `superpowers:verification-before-completion`
  6. commit the checkpoint as its own `cm`
- Do not start the next checkpoint until the previous checkpoint has passed review and been committed.
- If a reviewer recommendation is technically wrong for this codebase, document the reasoning during the `receiving-code-review` step and keep the checkpoint blocked until the disagreement is resolved.

---

## Stage 1: Eliminate Measured Hotspots Before Broader Restructure

### Task 1: Stabilize the stream render path

**Files:**
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/ClaudeStreamIndicator.tsx`
- Modify: `src/components/MessagePanel/CodexStreamIndicator.tsx`

- [ ] **Step 1: Remove transient stream indicators from Virtuoso row count**

Stop treating Claude/Codex stream indicators as regular timeline items that change `totalCount`. Render them in a dedicated stream rail outside the virtualized list so stream start/stop does not relayout the whole timeline.

- [ ] **Step 2: Narrow stream indicator subscriptions**

Replace whole-slice subscriptions with smaller selectors so Claude/Codex indicator renders only track the fields they actually display.

- [ ] **Step 3: Cut repeated derived string work**

Memoize expensive stream-tail display logic such as Claude preview tail trimming and any Codex reasoning / command-output tail shaping.

- [ ] **Step 4: Verify stream UI behavior**

Manual check: start a long Claude stream and a long Codex stream, confirm the message list stays stable and no extra blank row churn appears.

### Task 2: Fix markdown and message row hot rendering

**Files:**
- Modify: `src/components/MessagePanel/MessageBubble.tsx`
- Modify: `src/components/MessageMarkdown.tsx`

- [ ] **Step 1: Add memo boundaries at the message row level**

Wrap `MessageBubble` in `React.memo` with a comparison keyed to stable message identity so unrelated shell updates do not cause markdown rows to redraw.

- [ ] **Step 2: Add a plain-text fast path**

Detect content that does not need markdown parsing and render it through a lightweight text path instead of `react-markdown`.

- [ ] **Step 3: Memoize markdown transformation**

Cache cleaned content and markdown render inputs by `content` so repeated parent renders do not repeatedly parse the same message.

- [ ] **Step 4: Verify message rendering isolation**

Add or update a focused render-count test proving that stream state changes do not force existing message rows to fully rerender.

### Task 3: Fix Claude stream preview correctness and reduce Rust-side emit pressure

**Files:**
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src-tauri/src/daemon/claude_sdk/event_handler_stream.rs`
- Add/Modify: focused tests under `src-tauri/src/daemon/claude_sdk/`

- [ ] **Step 1: Restore frontend Claude preview accumulation**

Change the `preview` branch in `handleClaudeStreamEvent` from a no-op to actual preview accumulation with a bounded preview cap.

- [ ] **Step 2: Batch Claude `text_delta` preview emission in Rust**

Introduce a short batching window for `content_block_delta -> Preview` so a long stream stops emitting one GUI update per text fragment.

- [ ] **Step 3: Keep final-result semantics unchanged**

Preserve the existing SDK result / bridge ownership rules. This batching work must only change preview cadence, not final message routing.

- [ ] **Step 4: Verify batching with tests**

Add a test around the batching window or equivalent helper so the behavior is deterministic and does not regress.

### Task 4: Virtualize diagnostics and trim low-value paint cost

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/index.css`
- Modify: `src/animations.css`
- Modify: `src/utilities.css`

- [ ] **Step 1: Virtualize the logs tab**

Replace the current full log render with a virtualized list so diagnostics scale beyond the current in-memory cap without turning into DOM churn.

- [ ] **Step 2: Pre-format or centralize log timestamp formatting**

Stop instantiating `Date` and locale formatting logic in every log row render.

- [ ] **Step 3: Remove continuous decorative motion from hot surfaces**

Cut or sharply reduce:
- animated gradient heading behavior
- repeated pulse-only emphasis
- extra `backdrop-blur`
- broad `transition-all`

Preserve only purposeful motion for reveal, explicit action feedback, and pending state.

- [ ] **Step 4: Verify the visual reduction does not break affordance**

Manual check: provider cards, composer, dropdowns, and progress states still read clearly after removing decorative effects.

### Task 5: Split heavy secondary code from the initial bundle

**Files:**
- Modify: `src/components/MessageMarkdown.tsx`
- Modify: `vite.config.ts`

- [ ] **Step 1: Lazy-load markdown rendering**

Move the markdown rendering path behind a lazy boundary so plain-text-heavy sessions do not pay the full markdown cost in the initial hot path.

- [ ] **Step 2: Add explicit chunking strategy**

Use `manualChunks` or equivalent Vite output settings so markdown-related dependencies become a secondary chunk instead of staying in the main application bundle.

- [ ] **Step 3: Rebuild and compare chunk output**

Track the main chunk and confirm that secondary UI logic is no longer bundled entirely into the startup path.

---

## Stage 2: Reshape State And Shell Around Primary User Intent

### Task 6: Split store responsibilities and isolate shell updates

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/task-store/index.ts`
- Modify: `src/components/AgentStatus/index.tsx`
- Modify: `src/components/TaskPanel/index.tsx`

- [ ] **Step 1: Separate shell, chat, stream, and diagnostics responsibilities**

Refactor bridge-facing state so the top-level shell stops subscribing to broad message and stream slices when it only needs connection and context state.

- [ ] **Step 2: Reduce cross-domain rerender coupling**

Ensure task/session changes do not redraw the timeline, and stream changes do not redraw provider panels unless they intentionally share state.

- [ ] **Step 3: Reevaluate normalized collections after Stage 1**

If message volume and rerender profiling still justify it, introduce normalized `ids + byId` storage for messages and diagnostics. This remains conditional, not mandatory.

### Task 7: Redesign the shell into an IDE-style primary flow

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/AgentStatus/index.tsx`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/ReplyInput.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Add/Modify: new shell components under `src/components/` if needed

- [ ] **Step 1: Replace the current control-first shell**

Move toward:
- top context bar for active task/workspace/review state
- central conversation timeline
- right-side inspector for sessions/artifacts/advanced controls
- bottom diagnostics drawer for logs and runtime details

- [ ] **Step 2: Collapse provider controls into progressive disclosure**

Claude and Codex should default to compact status surfaces. Model, effort, history, and advanced config belong in secondary panels, sheets, or drawers.

- [ ] **Step 3: Demote secondary runtime detail**

Approvals, logs, reasoning, command output, and runtime hints should not visually compete with final assistant replies. Keep them contextual and expandable.

- [ ] **Step 4: Simplify the composer around the primary action**

Keep target selection and task context available but visually lightweight. The composer should read as the primary action surface, not a toolbar fragment inside a diagnostics app.

### Task 8: Finish the visual transition to a professional IDE hierarchy

**Files:**
- Modify: `src/index.css`
- Modify: `src/animations.css`
- Modify: `src/utilities.css`
- Modify: provider panels, message panel, reply input, and status components as needed

- [ ] **Step 1: Replace glow-first styling with hierarchy-first styling**

Use accent color for identity and state, not as a constant effect layer.

- [ ] **Step 2: Tighten spacing, badges, and typography**

Reduce ornament, shorten labels, simplify status treatment, and improve scanability of the conversation and task context.

- [ ] **Step 3: Follow IDE-style progress semantics**

Background work should appear as discreet progress by default. Only blocking or action-required states should escalate visually.

---

## Verification And Regression Protection

### Task 9: Add targeted verification around the actual bottlenecks

**Files:**
- Add/Modify: focused Rust tests under `src-tauri/src/daemon/`
- Add/Modify: frontend store/component tests under `tests/` or colocated test files

- [ ] **Step 1: Add stream-path regression tests**

Cover:
- Claude preview accumulation
- Claude preview batching
- stream rail lifecycle
- absence of Virtuoso row churn from transient indicators

- [ ] **Step 2: Add render-isolation checks**

Use render-count or profiler-style tests to prove stream updates do not redraw unrelated message rows, sidebars, or composer surfaces.

- [ ] **Step 3: Add diagnostics rendering checks**

Validate log virtualization and bounded diagnostics behavior.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
bun test
bun x tsc --noEmit
bun run build
```

- [ ] **Step 5: Perform real desktop smoke verification**

Exercise:
- connect Claude
- connect Codex
- send messages
- stream long responses
- open diagnostics
- open approvals
- recover history

Confirm the app still feels stable under sustained activity and the redesigned shell still supports the same workflows.

---

## Execution Checkpoints

### Checkpoint 1: Stream rail stabilization

**Scope:** Stage 1 / Task 1

- [ ] Implement the dedicated stream rail outside the virtualized timeline.
- [ ] Narrow Claude/Codex stream indicator subscriptions and trim repeated derived string work.
- [ ] Run focused frontend tests covering stream rail lifecycle and absence of `Virtuoso` row churn.
- [ ] Request deep code review for only this diff.
- [ ] Apply review feedback until no blocking or important findings remain.
- [ ] Re-run the focused tests plus `bun x tsc --noEmit`.
- [ ] Commit this checkpoint.

### Checkpoint 2: Message row and markdown hot path

**Scope:** Stage 1 / Task 2

- [ ] Add `React.memo` boundaries at the message row level.
- [ ] Add the plain-text fast path and memoized markdown transformation path.
- [ ] Run focused tests proving stream updates do not redraw existing message rows.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the focused tests plus `bun x tsc --noEmit`.
- [ ] Commit this checkpoint.

### Checkpoint 3: Claude preview pipeline correctness

**Scope:** Stage 1 / Task 3

- [ ] Restore frontend preview accumulation.
- [ ] Add Rust-side batching for Claude preview emission without changing final-result semantics.
- [ ] Run focused Rust and frontend tests for preview correctness and batching cadence.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the focused Rust/frontend tests.
- [ ] Commit this checkpoint.

### Checkpoint 4: Diagnostics rendering and hot-surface paint reduction

**Scope:** Stage 1 / Task 4

- [ ] Virtualize the logs tab and centralize log timestamp formatting.
- [ ] Remove or reduce low-value hot-surface motion and heavy effect usage.
- [ ] Run targeted UI tests and manual diagnostics checks.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the focused tests and manual smoke checks.
- [ ] Commit this checkpoint.

### Checkpoint 5: Bundle splitting and startup path cleanup

**Scope:** Stage 1 / Task 5

- [ ] Lazy-load the markdown-heavy path and add explicit chunking in `vite.config.ts`.
- [ ] Rebuild and compare bundle output against the current baseline.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run `bun run build` and `bun x tsc --noEmit`.
- [ ] Commit this checkpoint.

### Checkpoint 6: Store isolation and shell rerender boundaries

**Scope:** Stage 2 / Task 6

- [ ] Split shell, chat, stream, and diagnostics responsibilities enough to stop top-level layout churn.
- [ ] Reevaluate whether normalized collections are still necessary after the Stage 1 gains.
- [ ] Run targeted render-isolation and state-update regression tests.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the focused tests plus `bun x tsc --noEmit`.
- [ ] Commit this checkpoint.

### Checkpoint 7: Shell information architecture redesign

**Scope:** Stage 2 / Task 7, Steps 1-2

- [ ] Replace the control-first shell with the new context bar, central timeline, right inspector, and bottom diagnostics drawer structure.
- [ ] Collapse provider controls into progressive disclosure surfaces.
- [ ] Run manual smoke checks for Claude, Codex, task context, and diagnostics access.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the targeted smoke checks and `bun x tsc --noEmit`.
- [ ] Commit this checkpoint.

### Checkpoint 8: Interaction hierarchy and visual system pass

**Scope:** Stage 2 / Task 7, Steps 3-4 and Stage 2 / Task 8

- [ ] Demote secondary runtime detail and simplify the composer around the primary action.
- [ ] Finish the visual transition to hierarchy-first IDE styling.
- [ ] Run manual UX checks for readability, hierarchy, and action clarity.
- [ ] Request deep code review for this checkpoint.
- [ ] Apply and verify review feedback until clear.
- [ ] Re-run the targeted smoke checks and any updated frontend tests.
- [ ] Commit this checkpoint.

### Checkpoint 9: Final regression sweep

**Scope:** Task 9 and whole-plan verification

- [ ] Run the full regression suite and complete the real desktop smoke verification.
- [ ] Request one final deep code review covering the final integration diff.
- [ ] Apply and verify any remaining findings until the branch is clear.
- [ ] Re-run full verification with fresh evidence.
- [ ] Commit the final regression and polish changes, if any remain after review.

---

## Legacy Task Order Reference

Implement in this order unless a failing test or dependency forces reordering:

1. Stage 1 / Task 1: stream render stabilization
2. Stage 1 / Task 2: markdown and message row memoization
3. Stage 1 / Task 3: Claude preview correctness + batching
4. Stage 1 / Task 4: diagnostics virtualization + visual noise reduction
5. Stage 1 / Task 5: bundle splitting
6. Re-measure build output and manual responsiveness
7. Stage 2 / Task 6: store and shell isolation
8. Stage 2 / Task 7: shell redesign
9. Stage 2 / Task 8: final visual hierarchy pass
10. Task 9: full verification and desktop smoke test

---

## Acceptance Criteria

- Every checkpoint lands as a separately reviewed and freshly verified commit.
- The shell remains responsive while Claude or Codex streams continuously.
- Stream start/stop no longer causes the virtualized message list to relayout as if a new row was inserted or removed.
- Claude preview text actually appears in the UI and updates at a controlled cadence.
- Existing message rows do not repeatedly reparse markdown on unrelated stream updates.
- Logs and diagnostics scale without freezing the UI.
- Heavy secondary features move out of the initial hot path bundle.
- The visual design reads as a professional IDE, not a diagnostics dashboard with equal-weight panels.
- Background progress is discreet by default, while blocking or action-required states remain obvious.
- Claude/Codex connection, history, approvals, and task context remain fully functional after the redesign.

---

## Assumptions

- The desktop Tauri shell remains the primary product surface.
- Claude stays on the `--sdk-url` runtime path and Codex stays on app-server.
- The bridge-assisted MCP sidecar remains part of the runtime model during this optimization pass.
- This plan is allowed to break current layout continuity in service of a substantially better product shape.
- Mobile/responsive redesign is not part of this pass.
