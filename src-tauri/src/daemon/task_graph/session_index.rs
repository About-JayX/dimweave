use super::store::TaskGraphStore;
use super::types::*;

impl TaskGraphStore {
    /// All sessions belonging to a task.
    pub fn sessions_for_task(
        &self,
        task_id: &str,
    ) -> Vec<&SessionHandle> {
        self.sessions
            .values()
            .filter(|s| s.task_id == task_id)
            .collect()
    }

    /// Child sessions whose parent is the given session.
    pub fn children_of_session(
        &self,
        parent_session_id: &str,
    ) -> Vec<&SessionHandle> {
        self.sessions
            .values()
            .filter(|s| {
                s.parent_session_id.as_deref() == Some(parent_session_id)
            })
            .collect()
    }

    /// The lead session for a task (if set on the task record).
    pub fn lead_session_for_task(
        &self,
        task_id: &str,
    ) -> Option<&SessionHandle> {
        let task = self.tasks.get(task_id)?;
        let lead_id = task.lead_session_id.as_deref()?;
        self.sessions.get(lead_id)
    }
}
