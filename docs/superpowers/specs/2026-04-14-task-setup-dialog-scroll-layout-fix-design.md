# Task Setup Dialog Scroll Layout Fix Design

> **Scope:** Visual/layout-only follow-up on the stage-complete `task_agents[]` task setup dialog. No task-agent data model or routing changes.

> **Context:** The current dialog scrolls as one large container. That causes the action buttons to move with the content and leaves the scrollbar visually attached to the outer modal shell instead of the lower provider-panel area.

## Goal

Make the `TaskSetupDialog` behave like a structured modal:

- the action bar (`Cancel`, `Create & Connect`, `Create` / `Save`) stays fixed at the bottom
- only the lower provider-panel section scrolls
- the scrollbar belongs to that inner scroll region and has a cleaner visual style

## Non-Goals

- No task-agent CRUD, routing, launch, or persistence changes
- No copy changes beyond what layout requires
- No `happy-dom` dependency changes

## Current Problem

`TaskSetupDialog` currently puts `overflow-y-auto max-h-[90vh]` on the modal container itself. That means:

- the whole dialog scrolls as one unit
- the button row moves away from the bottom when content grows
- the scrollbar appears on the outer shell, which reads as a layout bug

## Desired Behavior

### Dialog Structure

The modal should be split into three vertical sections:

1. Header and `Agents` editor region
2. Scrollable provider-panel region
3. Fixed bottom action bar

### Scrolling

- The outer dialog should no longer be the scroll container
- The middle provider-panel region should be the only vertical scroll container
- The bottom action bar must remain visible while scrolling

### Scrollbar Styling

The scroll region should use a subtle custom scrollbar treatment:

- thinner track
- rounded thumb
- subdued track/background
- slightly stronger thumb contrast so the scroll affordance remains visible

The intent is not decorative styling; it is to remove the current “broken outer scrollbar” look.

## Implementation Shape

### `TaskSetupDialog.tsx`

- Move outer modal container from scrollable block to fixed-height flex column
- Keep top `Agents` block outside the scrolling region
- Wrap `AgentStatusPanel` in an inner scroll area with its own overflow rules
- Move the action buttons into a footer container pinned to the bottom of the modal body
- Apply scoped scrollbar utility/class styling directly to the inner scroll region

### `TaskSetupDialog.test.tsx`

Add assertions for:

- outer dialog shell no longer being the scroll container
- presence of a dedicated inner scroll region for provider panels
- footer action bar still rendering as a separate bottom section

## Risks

- If the fixed-height split is too aggressive, shorter viewports can compress the top `Agents` area uncomfortably
- If the inner scroll region min-height is wrong, the footer can overlap provider panels

The implementation should therefore use a bounded max-height dialog plus a flex child with `min-h-0` on the scroll region.

## Acceptance Criteria

- `Cancel`, `Create & Connect`, and `Create` stay fixed at the bottom of the dialog
- the dialog shell itself is not the vertical scroll container
- only the lower provider-panel section scrolls
- scrollbar styling is visibly cleaner and attached to the inner scroll region
- existing dialog behavior and copy remain otherwise unchanged
