# Task Pane Card-Only Simplification Design

> **Status:** Accepted

> **Context:** The task pane already supports multi-task listing, `task_agents[]` is already the sole task-agent truth source, and task persistence is now backed by SQLite with explicit `project_root` and `task_worktree_root`.

## Goal

Simplify the left task pane so each task is represented by a single compact task card:

- keep the task summary card
- remove the separate `Agents` panel
- remove the separate `Sessions` panel
- remove the separate `Artifacts` panel
- keep multi-task newest-first list behavior
- keep `activeTaskId` as the source of truth for message-panel sync

## Why Change It

The current pane still carries three stacked detail sections below the task card:

- `Agents`
- `Sessions`
- `Artifacts`

In practice this creates two problems:

- the pane is visually heavy for the main job it actually performs, which is task switching
- `Agents` duplicates information already summarized in the task card via agent pills

`Sessions` and `Artifacts` are not core navigation or task-switching surfaces. They are secondary inspection surfaces layered into the pane, which makes the task list harder to scan.

## Product Decision

The task pane becomes a compact task-card list.

Each task entry shows only:

- title
- task id
- status badge
- saved state indicator
- `Edit Task`
- agent pills

The task pane no longer shows:

- `Agents` list rows
- `Sessions`
- `Artifacts`

`New Task` remains at the list level.

## Interaction Model

### Task List

- all tasks for the selected `projectRoot` remain visible
- ordering stays newest-first by creation time
- clicking a task card sets `activeTaskId`
- the active task remains the only visually expanded/selected card state
- the message panel continues to follow `activeTaskId`

### Agent Management

Removing the `Agents` panel also removes inline add/edit/remove/reorder affordances from the task pane.

The replacement rule is:

- `Edit Task` becomes the only task-pane entry point for changing agents

That keeps the pane dense and focused. Agent management still exists, but it moves behind one intentional action instead of occupying permanent vertical space.

## Sessions And Artifacts

This change removes the `Sessions` and `Artifacts` sections from the task pane only.

It does **not** remove:

- session state from the store/backend
- artifact state from the store/backend
- daemon events that keep those states updated
- artifact/session persistence

Those data flows remain valid and available for future surfaces if needed. This is a pane simplification, not a data-model removal.

## Architecture

### Component Boundaries

The preferred direction is:

- `TaskPanel/index.tsx` becomes a compact task-card list container
- `TaskHeader.tsx` becomes the full visible task card surface for both active and inactive tasks
- `TaskAgentList`, `SessionTree`, and `ArtifactTimeline` stop rendering from the task pane path

The implementation should avoid introducing a second card component unless the current `TaskHeader` becomes unreasonably overloaded.

### State

No new state model should be introduced.

Keep:

- `activeTaskId` as the selected task source
- `selectedWorkspace` / `projectRoot` filtering as currently implemented

Do not add a new accordion expansion state. The pane is becoming simpler, not more stateful.

## Non-Goals

- deleting session or artifact persistence
- redesigning message routing
- changing task ordering away from newest-first
- adding new quick actions into the task card beyond what already exists
- moving `Sessions` or `Artifacts` to a new surface in this change

## Risks

### Risk 1: Accidental behavior loss in task switching

The pane simplification must not break the current multi-task list or active-task message sync.

Mitigation:

- keep `activeTaskId` unchanged as the only selection source
- add focused regression tests for active-task switching and list rendering

### Risk 2: Dead task-pane-only code paths are left partially referenced

`TaskPanel/index.tsx` currently wires sessions, artifacts, and artifact detail hooks directly.

Mitigation:

- remove those render-path imports from `TaskPanel/index.tsx`
- either leave orphaned components unused or delete them in-scope if the plan explicitly authorizes it
- do not partially keep task-pane-only plumbing alive in the card-only path

## Acceptance Criteria

- the task pane renders only the compact task card for each task
- the separate `Agents` panel is gone
- the separate `Sessions` panel is gone
- the separate `Artifacts` panel is gone
- `Edit Task` remains available from the task card
- multi-task list behavior remains newest-first and active-task selection still drives the message panel
