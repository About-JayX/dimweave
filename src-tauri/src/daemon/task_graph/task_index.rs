use super::store::TaskGraphStore;
use super::types::*;

impl TaskGraphStore {
    /// All tasks for a given workspace root.
    /// Matches on `project_root` so tasks with different worktrees
    /// still appear in the same project list.
    pub fn tasks_for_workspace(&self, workspace_root: &str) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.project_root == workspace_root)
            .collect()
    }

    /// The most recent non-terminal task for a workspace.
    /// Matches on `project_root` so worktree assignment doesn't hide tasks.
    pub fn active_task(&self, workspace_root: &str) -> Option<&Task> {
        self.tasks
            .values()
            .filter(|t| {
                t.project_root == workspace_root
                    && !matches!(t.status, TaskStatus::Done | TaskStatus::Error)
            })
            .max_by_key(|t| t.updated_at)
    }
}
