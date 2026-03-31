use crate::daemon::task_graph::types::SessionRole;

/// Provider-agnostic parameters for registering a normalized session.
pub struct SessionRegistration {
    pub task_id: String,
    pub parent_session_id: Option<String>,
    pub role: SessionRole,
    pub cwd: String,
    pub title: String,
    /// Provider-specific external ID (e.g. Codex thread_id, Claude session_id).
    pub external_id: Option<String>,
}
