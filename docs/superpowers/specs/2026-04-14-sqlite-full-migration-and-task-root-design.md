# SQLite Full Migration And Task Root Split Design

> **Supersedes:** [2026-04-14-task-root-split-and-agent-dnd-design.md](2026-04-14-task-root-split-and-agent-dnd-design.md)

> **Reason for replacement:** The original follow-up only fixed task root semantics and agent drag/drop. The current requirements now explicitly require replacing the JSON persistence model with SQLite across all persisted domains, so the smaller plan is no longer sufficient.

## Goal

Replace the current JSON-based persistence model with a single SQLite-backed persistence layer, while also fixing the overloaded task path model that breaks multi-task lists.

This design covers:

- full replacement of JSON persistence with SQLite
- explicit split between stable project root and per-task worktree root
- repair of multi-task list behavior
- stabilization of agent reorder persistence against the new storage model

Telegram routing ownership is explicitly out of scope for this round. Only its persisted config/state storage moves from JSON to SQLite.

## Problems To Solve

### 1. JSON persistence is no longer adequate

The app currently persists different domains through unrelated JSON files and snapshots:

- task graph JSON
- daemon snapshot JSON
- Telegram config JSON
- Feishu Project config JSON
- Feishu Project inbox JSON

This worked while the model was smaller. It is now a liability:

- no real query layer
- poor transactional guarantees
- difficult schema evolution
- multiple persistence formats for related state
- harder to debug and validate consistency

### 2. Task path semantics are overloaded

The current task model uses a single path field for two different concepts:

- the stable project root the user selected
- the task-specific worktree path used to run that task

That is why multi-task lists collapse after creating another task: once the active task snapshot overwrites frontend workspace state with the task worktree path, task list filtering no longer matches older tasks from the same project.

## Storage Direction

### Single SQLite Database

Use one SQLite database file for the whole persisted app state.

Recommended location:

- platform config directory
- same app config root currently used by JSON files
- filename such as `state.sqlite3`

This round intentionally uses one shared database file. We are not introducing instance-specific app ids or per-channel database isolation.

### No JSON Migration

Old JSON files are not migrated.

Rules:

- the new version only reads SQLite
- existing JSON files are ignored
- new writes only go to SQLite

This is an explicit reset of persisted state rather than a compatibility migration.

## Data Model

### Tasks

Tasks must split path semantics into two columns:

- `project_root`
- `task_worktree_root`

Rules:

- task list/grouping/filtering uses `project_root`
- runtime launch/history/worktree logic uses `task_worktree_root`
- newly created tasks start with `project_root = selected project`
- after worktree creation, `task_worktree_root` is updated to the isolated task path
- `project_root` never changes because of worktree creation

### Task Graph Tables

The SQLite schema should cover at least:

- `tasks`
- `task_agents`
- `sessions`
- `artifacts`
- `buffered_messages`
- `meta`

`meta` stores at least:

- schema version

### External Integration Tables

Persist these in the same database:

- `telegram_config`
- `feishu_project_config`
- `feishu_project_inbox`

This removes JSON files for those domains.

## Architecture

### Persistence Boundary

Create one SQLite persistence layer instead of having each module manage its own JSON file format.

The daemon should depend on one persistence service / module responsible for:

- opening the database
- applying schema initialization
- loading state into in-memory structures where needed
- writing updates transactionally

### In-Memory vs Persisted

Not every runtime field belongs in SQLite.

Persist:

- task graph data
- buffered messages
- integration config/state that must survive restart

Keep transient runtime-only state in memory:

- active sockets
- launch channels
- current websocket handles
- ephemeral in-process runtime flags

This is a persistence migration, not a “database everything” rewrite.

## UI / Frontend Impact

Frontend DTOs and store hydration must switch from:

- `workspaceRoot` as overloaded task path

to:

- stable `projectRoot` for grouping and selected workspace alignment
- `taskWorktreeRoot` where task-specific runtime/workspace detail is actually needed

Task list behavior after the fix:

- creating a new task in a project does not hide older tasks from that same project
- selected workspace remains aligned to `projectRoot`
- accordion list continues to work off project-grouped tasks

## Agent Reorder

Keep the existing reorder command semantics, but ensure they remain correct with SQLite-backed persistence.

This means:

- reorder writes must be transactionally persisted
- reload must restore agent order exactly
- drag UI must continue to call the same reorder intent, but now against SQLite-backed storage

## Non-Goals

- Telegram routing ownership redesign
- app-id based multi-instance isolation
- migration of old JSON data into SQLite
- redesigning task panel visuals beyond what the task-root fix requires
- replacing runtime in-memory ownership structures with database-backed live state

## Acceptance Criteria

- all current persisted domains move from JSON to SQLite
- old JSON files are no longer used
- tasks store separate `project_root` and `task_worktree_root`
- multi-task list no longer collapses when creating additional tasks in one project
- agent reorder persists correctly through SQLite-backed storage
- Telegram/Feishu config persistence no longer relies on standalone JSON files
