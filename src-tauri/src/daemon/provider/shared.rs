use crate::daemon::task_graph::types::{Provider, SessionRole, SessionStatus};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHistoryEntry {
    pub provider: Provider,
    pub external_id: String,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub cwd: Option<String>,
    pub archived: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHistoryPage {
    pub entries: Vec<ProviderHistoryEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResumeTarget {
    pub role: SessionRole,
    pub cwd: String,
    pub external_id: String,
}
