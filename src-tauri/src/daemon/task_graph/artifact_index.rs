use super::store::TaskGraphStore;
use super::types::*;

impl TaskGraphStore {
    /// All artifacts for a task, ordered by creation time.
    pub fn artifacts_for_task(
        &self,
        task_id: &str,
    ) -> Vec<&Artifact> {
        let mut arts: Vec<_> = self
            .artifacts
            .values()
            .filter(|a| a.task_id == task_id)
            .collect();
        arts.sort_by_key(|a| a.created_at);
        arts
    }

    /// All artifacts produced by a specific session.
    pub fn artifacts_for_session(
        &self,
        session_id: &str,
    ) -> Vec<&Artifact> {
        let mut arts: Vec<_> = self
            .artifacts
            .values()
            .filter(|a| a.session_id == session_id)
            .collect();
        arts.sort_by_key(|a| a.created_at);
        arts
    }
}
