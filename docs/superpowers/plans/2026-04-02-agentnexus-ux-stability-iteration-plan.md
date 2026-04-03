# Dimweave UX Stability Iteration Plan

> **For agentic workers:** REQUIRED SUB-SKILLS: Use `superpowers:subagent-driven-development` (preferred) or `superpowers:executing-plans` to implement this plan checkpoint-by-checkpoint. Every checkpoint must also run the `superpowers:requesting-code-review`, `superpowers:receiving-code-review`, and `superpowers:verification-before-completion` workflow before commit. Steps use checkbox syntax for tracking.

**Goal:** Fix the next layer of post-rebuild issues: abrupt chat surfaces, noisy Codex startup errors, oversized header context, unnecessary empty-state copy, and stream rendering that still feels choppy under load.

**Recommended Approach:** Do a stability-first iteration inside the current `perf-ux-rebuild` shell rather than another broad shell rewrite. The right move here is to tighten the hot runtime paths and soften the UI surfaces that now feel too aggressive, while preserving the new information architecture.

**Why This Approach:** The user-reported issues are not random polish requests. They cluster into three root causes:

1. **Expected startup races are still surfaced as user-facing errors.**
2. **High-frequency stream updates still paint too often on both the Rust and React sides.**
3. **The new shell hierarchy is directionally better, but some surfaces still carry too much visual weight.**

**Tech Stack:** Tauri 2, Rust/Tokio daemon, React 19, Zustand, Tailwind 4, Bun, Cargo

---

## Deep Review Findings

### 1. Codex `Connection refused` is a startup race, not a true user-action failure

Evidence chain:

- The UI fetches provider history as soon as Claude/Codex panels mount for a workspace:
  - `src/components/ClaudePanel/index.tsx`
  - `src/components/AgentStatus/CodexPanel.tsx`
- The task store currently has **no in-flight dedupe** for workspace history requests:
  - `src/stores/task-store/index.ts`
- The daemon history endpoint always tries `Codex thread/list` on port `4500`, even when Codex is not running yet:
  - `src-tauri/src/daemon/provider/history.rs`
- The WebSocket client emits `[Codex] connect failed: ...` on every refused socket open:
  - `src-tauri/src/daemon/codex/ws_client.rs`

Observed symptom in the user log:

- `15:59:07` and `15:59:15` show connection-refused errors
- Codex is only actually spawned at `15:59:17`
- Codex becomes ready at `15:59:25`

Conclusion:

- These logs are mostly expected during cold start and history hydration.
- They should not currently be shown as red runtime failures to the user.
- The system needs both **startup-aware fallback behavior** and **UI fetch dedupe**.

### 2. Stream rendering is still too chatty

Evidence chain:

- Claude preview batching exists, but frontend still paints preview text directly on each event:
  - `src/stores/bridge-store/stream-reducers.ts`
  - `src/components/MessagePanel/ClaudeStreamIndicator.tsx`
- Codex emits reasoning, command output, and agent-message delta updates at high frequency:
  - `src-tauri/src/daemon/codex/session_event.rs`
  - `src-tauri/src/daemon/gui.rs`
- The frontend currently writes every stream event straight into Zustand:
  - `src/stores/bridge-store/listener-setup.ts`
- Claude preview also forces `scrollTop = scrollHeight` on every preview update:
  - `src/components/MessagePanel/ClaudeStreamIndicator.tsx`

Conclusion:

- The message list is more stable than before, but the stream rail itself is still updating too often.
- This is why the app can feel “laggy” or “sticky” during long streaming turns even though the main timeline no longer churns.

### 3. Message bubbles and stream cards are visually too loud

Evidence chain:

- Message bubbles still use strong border-and-fill treatment on both user and assistant messages:
  - `src/components/MessagePanel/MessageBubble.tsx`
- Source badges still carry glow/shadow accents:
  - `src/components/MessagePanel/SourceBadge.tsx`
- Stream indicators visually read like new messages instead of “live draft state”:
  - `src/components/MessagePanel/ClaudeStreamIndicator.tsx`
  - `src/components/MessagePanel/CodexStreamIndicator.tsx`

Conclusion:

- The shell hierarchy was simplified, but the actual reply surfaces still feel abrupt.
- The stream rail needs to look like a live working state, not a second conversation lane.

### 4. The header is still overcommitted

Evidence chain:

- `ShellContextBar` dedicates a large always-visible center slot to task context:
  - `src/components/ShellContextBar.tsx`
- When there is no active task, the app renders a long explanatory sentence:
  - `src/components/ShellContextBar.tsx`

Conclusion:

- The shell would read better if the context block became a compact left-side trigger that opens a popover or sheet, rather than always occupying the header.
- The “No active task selected...” copy is unnecessary at this level; absence is enough.

---

## Rejected Alternatives

### Alternative A: quick visual-only patch

Reject. Softening the bubble colors alone would not remove the Codex startup errors or the stream update choppiness.

### Alternative B: another full shell rewrite

Reject. The current shell direction is already usable. The problem now is refinement and runtime behavior, not a missing layout concept.

### Alternative C: runtime abstraction rewrite before fixing symptoms

Reject for now. The current evidence does not justify replacing the entire Tauri event path yet. We still have simpler fixes with higher confidence:

- startup-aware history behavior
- in-flight fetch dedupe
- stream event coalescing
- softer message surfaces

---

## Iteration Checkpoints

### Task 1: Quiet Codex history hydration and remove false startup errors

**Files:**
- Modify: `src/stores/task-store/index.ts`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`
- Modify: `src-tauri/src/daemon/provider/history.rs`
- Modify: `src-tauri/src/daemon/codex/ws_client.rs`
- Add/Modify: focused tests under `tests/` and `src-tauri/src/daemon/provider/`

- [ ] **Step 1: Write a failing test for task-store provider-history dedupe**

Add a focused test proving repeated `fetchProviderHistory(workspace)` calls reuse the same in-flight fetch instead of hitting the backend multiple times.

- [ ] **Step 2: Add in-flight dedupe to `fetchProviderHistory`**

Cache pending workspace requests in the task store so ClaudePanel and CodexPanel do not both trigger fresh history RPCs for the same workspace during mount or reconnect.

- [ ] **Step 3: Make daemon-side Codex history lookup startup-aware**

Before calling remote `thread/list`, check whether Codex is actually reachable or whether the system should immediately use the local fallback. Treat “port not listening yet” as expected startup state, not as a user-facing error.

- [ ] **Step 4: Downgrade or suppress expected cold-start connection-refused logs**

Keep real transport failures visible, but stop emitting red `[Codex] connect failed` noise for the expected “history requested before app-server is ready” path.

- [ ] **Step 5: Verify the startup path**

Manual check:
- launch app with Claude only
- open workspace
- confirm Codex history still loads from fallback
- confirm no repeated red `Connection refused` logs appear before Codex starts

### Task 2: Coalesce stream updates so the rail feels fluid

**Files:**
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/stream-reducers.ts`
- Modify: `src/components/MessagePanel/ClaudeStreamIndicator.tsx`
- Modify: `src/components/MessagePanel/CodexStreamIndicator.tsx`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Add/Modify: `src/stores/bridge-store/listener-setup.test.ts`
- Add/Modify: focused Rust tests under `src-tauri/src/daemon/codex/`

- [ ] **Step 1: Write failing tests for frontend stream coalescing**

Add tests showing that multiple Claude/Codex stream events inside a short frame window are merged into one visible state update.

- [ ] **Step 2: Batch Codex reasoning / command output / delta emission in Rust**

Introduce short-window coalescing so Codex does not emit every reasoning fragment and output fragment as an independent GUI event.

- [ ] **Step 3: Coalesce frontend stream application to animation-frame cadence**

Do not call `set(...)` for every stream payload immediately. Buffer transient stream updates briefly and flush them once per frame or per short window, while keeping final messages and turn completion immediate.

- [ ] **Step 4: Remove per-update forced scrolling**

Replace direct `scrollTop = scrollHeight` on every Claude preview update with a throttled or RAF-based bottom lock. The stream rail should feel live, not fight the browser layout engine.

- [ ] **Step 5: Verify with a long stream**

Manual check:
- run long Claude preview
- run long Codex reasoning / command output
- confirm the rail updates smoothly without obvious jank or sticky scroll behavior

### Task 3: Redesign message bubbles and stream surfaces to reduce abruptness

**Files:**
- Modify: `src/components/MessagePanel/MessageBubble.tsx`
- Modify: `src/components/MessagePanel/SourceBadge.tsx`
- Modify: `src/components/MessagePanel/ClaudeStreamIndicator.tsx`
- Modify: `src/components/MessagePanel/CodexStreamIndicator.tsx`
- Modify: relevant shared styles under `src/animations.css` / `src/utilities.css` if needed
- Add/Modify: visual snapshot or view-model tests if practical

- [ ] **Step 1: Soften the base bubble chrome**

Reduce border contrast, cut message-surface glow, and give user / assistant messages a calmer visual relationship. Metadata should support the message, not dominate it.

- [ ] **Step 2: Demote badge intensity**

Tone down `SourceBadge` shadows and saturated outlines so the source chip reads as metadata rather than as a neon status token.

- [ ] **Step 3: Make stream surfaces visually distinct from committed replies**

Stream indicators should look like “live draft / runtime progress” rather than like another finalized chat bubble. Keep them lighter, flatter, and more obviously transient.

- [ ] **Step 4: Verify the before/after read path**

Manual review in three cases:
- user reply
- agent final reply
- agent live streaming state

Success means the eye lands on committed messages first, and the stream rail no longer feels abrupt.

### Task 4: Collapse header context into a compact left-side popover or sheet

**Files:**
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/App.tsx`
- Add: compact task/workspace popover or left sheet component under `src/components/`
- Modify: `src/components/MobileInspectorSheet.tsx` if shared behavior is useful
- Add/Modify: focused component tests

- [ ] **Step 1: Replace the always-expanded task block with a compact trigger**

Move task/workspace/session/artifact context behind a compact left-aligned trigger. Default header state should be slimmer and easier to scan.

- [ ] **Step 2: Remove the no-task explanatory sentence**

When there is no active task, do not render the long empty-state copy in the header. Use either no panel at all or a minimal neutral placeholder.

- [ ] **Step 3: Keep provider status in the header, but reduce visual competition**

Claude/Codex online state should remain visible, but the header should read as a status bar, not a second dashboard.

- [ ] **Step 4: Verify small and large window behavior**

Manual check:
- wide window
- narrow desktop window
- inspector open/closed

Success means task context is still reachable, but the header no longer feels heavy.

### Task 5: Final verification and interaction audit

**Files:**
- Modify: `docs/superpowers/plans/2026-04-02-agentnexus-performance-ux-results.md`

- [ ] **Step 1: Run fresh verification**

Run:
- `bun test`
- `bun x tsc --noEmit`
- `bun run build`
- any targeted Rust tests added for Codex history / stream batching

- [ ] **Step 2: Perform manual GUI verification**

Check this sequence end-to-end:
- cold start app
- connect Claude
- connect Codex
- confirm no startup history noise
- stream a long Claude response
- stream a long Codex response
- inspect the header popover flow
- inspect the revised bubble surfaces

- [ ] **Step 3: Update results documentation**

Append a new section to the results doc describing:
- what was fixed
- which startup logs were intentionally downgraded
- what changed in the stream rail
- what changed in the header and message surfaces

---

## Verification Commands

### Frontend

- `bun test`
- `bun x tsc --noEmit`
- `bun run build`

### Rust / daemon

- `cargo test --manifest-path src-tauri/Cargo.toml`
- targeted tests for any new helper in `src-tauri/src/daemon/provider/` or `src-tauri/src/daemon/codex/`

### Manual smoke

1. Start GUI from `perf-ux-rebuild`
2. Open a workspace with no active Codex runtime
3. Confirm provider history loads without red Codex startup noise
4. Connect Claude and verify `--sdk-url` session starts cleanly
5. Connect Codex and verify the connect button does not hang
6. Trigger long streaming responses and verify the rail remains visually smooth
7. Open the new left-side task context entry point and confirm the header remains compact

---

## Commit Strategy

Use one focused commit per checkpoint:

1. `Quiet Codex startup history noise`
2. `Coalesce stream rail updates`
3. `Soften message and stream surfaces`
4. `Collapse shell header context`
5. `Document UX stability iteration`

Do not batch all five into one commit.
