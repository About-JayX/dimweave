use super::store::TaskGraphStore;
use super::types::*;

impl TaskGraphStore {
    /// All tasks for a given workspace root.
    pub fn tasks_for_workspace(&self, workspace_root: &str) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.workspace_root == workspace_root)
            .collect()
    }

    /// The most recent non-terminal task for a workspace.
    pub fn active_task(&self, workspace_root: &str) -> Option<&Task> {
        self.tasks
            .values()
            .filter(|t| {
                t.workspace_root == workspace_root
                    && !matches!(t.status, TaskStatus::Done | TaskStatus::Error)
            })
            .max_by_key(|t| t.updated_at)
    }
}
