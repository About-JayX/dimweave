# Tools Drawer Design

## Summary

The current left-rail `Bug Inbox` entry is too narrow for the way the product now exposes external utilities. The user wants that rail item to read as a generic tool entry, use a wrench-style icon instead of a bug icon, and host both the Telegram tool and the Feishu Project inbox inside one drawer. Each tool area should be independently expandable and collapsible so the drawer can hold both surfaces without forcing them to stay open at the same time.

This design keeps the existing shell architecture, drawer mechanics, Telegram runtime wiring, and Feishu Project inbox workflow intact. It only changes the visual semantics of the rail entry and introduces a small frontend-only container that groups the two existing tool panels.

## Product Goal

- Replace the current bug-specific rail affordance with a generic tools affordance.
- Keep Feishu Project available from the same left-drawer region it uses today.
- Move Telegram out of the Agents drawer and into the new tools drawer.
- Let the user expand or collapse Telegram and Feishu sections independently.

## Scope

### Included

- Change the left rail `Bug Inbox` entry to a wrench-style `Tools` entry.
- Update the drawer header copy from bug-specific wording to tools wording.
- Add a new `ToolsPanel` container inside the existing drawer system.
- Render `TelegramPanel` and `BugInboxPanel` inside that container.
- Add disclosure controls so each section can be expanded or collapsed.
- Remove the standalone Telegram card from the Agents drawer.
- Add focused frontend tests for the renamed rail item and tools drawer sections.

### Excluded

- Renaming the internal `bugs` pane key or changing shell routing semantics.
- Changing Telegram runtime behavior, pairing flow, or persistence.
- Changing Feishu MCP sync behavior, inbox actions, or linked task handling.
- Adding new integrations beyond Telegram and Feishu Project.
- Reworking the rest of the Agents drawer.

## Project Memory

### Recent related commits

- `de6fcb99` — added the Bug Inbox shell panel and drawer slot.
- `6e8c79ed` — pivoted the Feishu inbox config UI to MCP connection fields.
- `c3b18acc` — polished Telegram panel loading/pairing behavior and task panel feedback.
- `9ef63dfb` — finalized the Bug Inbox frontend MCP workflow UI.

### Related plans and addenda

- `docs/superpowers/plans/2026-04-09-feishu-project-bug-inbox.md`
- `docs/superpowers/plans/2026-04-09-feishu-project-mcp-pivot.md`
- `docs/superpowers/plans/2026-04-09-fix-tg-and-task-panel.md`
- `docs/superpowers/plans/2026-04-09-fix-tg-and-task-panel-review-addendum.md`

### Constraints carried forward

- Preserve the existing shell drawer architecture (`ShellContextBar` + `TaskContextPopover`) instead of adding a new modal or floating settings surface.
- Preserve the existing Feishu inbox badge behavior so unread/active item count remains visible on the rail.
- Do not disturb Telegram pairing behavior or validation logic that was stabilized in the April 9 review addendum.
- Minimize scope by keeping the internal `bugs` pane key unchanged; the rename is visual, not architectural.

## Architecture

### Keep the existing shell pane identity, but rename the presentation

The lowest-risk implementation is to keep the internal `bugs` pane id as-is and only change the visible label/icon/copy to `Tools`. That preserves existing shell state transitions, drawer mounting behavior, and badge wiring without requiring a broader type or state migration.

### Introduce a dedicated `ToolsPanel` container

Instead of making `TaskContextPopover` directly render `BugInboxPanel`, the `bugs` pane should render a new `ToolsPanel`. That container will own only local disclosure state and section presentation. It does not fetch remote data itself; it composes the existing `TelegramPanel` and `BugInboxPanel`.

### Use independent disclosure sections

The drawer should show two vertical sections:

1. `Telegram`
2. `Feishu Project`

Each section has its own disclosure button and body. The default open state should preserve the old user path:

- `Feishu Project`: expanded by default
- `Telegram`: collapsed by default

This keeps the current bug-inbox workflow immediately visible while still surfacing Telegram in the same tool drawer.

### Remove Telegram from Agents to avoid duplication

Once Telegram appears in the tools drawer, the Agents drawer should stop rendering `TelegramPanel`. The Agents drawer remains focused on provider/runtime control instead of becoming a duplicate tools surface.

## Behavior Changes

### Left rail

- The rail item label becomes `Tools`.
- The rail icon becomes a wrench/tool icon.
- The badge still reflects the active Feishu inbox item count.

### Drawer header

- The `bugs` pane header copy changes from bug-specific wording to tools wording.
- The drawer body now hosts the two disclosure sections instead of the Feishu panel directly.

### Tools drawer content

- Telegram can be opened or collapsed without affecting Feishu.
- Feishu can be opened or collapsed without affecting Telegram.
- The existing Telegram and Feishu functionality remains unchanged once their section is open.

## File Plan

### Modified files

- `src/components/ShellContextBar.tsx`
- `src/components/TaskContextPopover.tsx`
- `src/components/AgentStatus/index.tsx`
- `src/components/BugInboxPanel/index.tsx` (only if embedding polish is required)
- `src/components/ShellContextBar.test.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/BugInboxPanel/index.test.tsx` (only if embedding polish changes markup)

### New files

- `src/components/ToolsPanel/index.tsx`
- `src/components/ToolsPanel/index.test.tsx`

## Testing Strategy

- Extend the shell rail test to assert the `Tools` label replaces `Bug Inbox`.
- Extend the drawer test to assert the tools header copy and section labels render.
- Add a focused `ToolsPanel` test that checks the default disclosure state (`Telegram` collapsed, `Feishu Project` expanded) and presence of disclosure controls.
- Re-run the focused frontend tests and `bun run build`.

## Acceptance Criteria

- The left rail entry now presents as `Tools` with a wrench-style icon.
- Opening that rail item shows a tools-oriented drawer header instead of bug-specific copy.
- The drawer contains `Telegram` and `Feishu Project` sections.
- Each section has an expand/collapse affordance.
- Telegram is no longer duplicated inside the Agents drawer.
- Focused frontend tests and `bun run build` pass.
