use serde::{Deserialize, Serialize};

/// Frontend DTO: active task with its sessions, artifacts, and runtime summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub task: crate::daemon::task_graph::types::Task,
    pub sessions: Vec<crate::daemon::task_graph::types::SessionHandle>,
    pub artifacts: Vec<crate::daemon::task_graph::types::Artifact>,
    pub provider_summary: Option<TaskProviderSummary>,
}

/// Frontend DTO: session tree for a task (flat list, tree via parent_session_id).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTreeSnapshot {
    pub task_id: String,
    pub sessions: Vec<crate::daemon::task_graph::types::SessionHandle>,
}

/// Frontend DTO: task history entry with summary counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub task: crate::daemon::task_graph::types::Task,
    pub session_count: usize,
    pub artifact_count: usize,
}

/// Structured snapshot of one online agent, used in query responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OnlineAgentInfo {
    pub agent_id: String,
    pub role: String,
    pub model_source: String,
}

/// Per-task provider binding summary (AC5).
/// Exposes which provider handles each role and whether it is currently online.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskProviderSummary {
    pub task_id: String,
    pub lead_provider: String,
    pub coder_provider: String,
    pub lead_online: bool,
    pub coder_online: bool,
}
