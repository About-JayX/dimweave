use serde::{Deserialize, Serialize};

/// Frontend DTO: active task with its sessions, artifacts, and runtime summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub task: crate::daemon::task_graph::types::Task,
    pub sessions: Vec<crate::daemon::task_graph::types::SessionHandle>,
    pub artifacts: Vec<crate::daemon::task_graph::types::Artifact>,
    #[serde(default)]
    pub task_agents: Vec<crate::daemon::task_graph::types::TaskAgent>,
    pub provider_summary: Option<TaskProviderSummary>,
    #[serde(default)]
    pub agent_runtime_statuses: Vec<TaskAgentRuntimeStatus>,
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

/// Per-agent runtime status within a task snapshot / event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskAgentRuntimeStatus {
    pub agent_id: String,
    pub online: bool,
}

/// Per-task provider binding summary (AC5).
/// Exposes which provider handles each role, whether it is currently online,
/// and the live provider session info for each role.
/// `lead_agent_id` / `coder_agent_id` carry the concrete `TaskAgent.agent_id`
/// when task_agents[] are present, falling back to provider name for legacy tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskProviderSummary {
    pub task_id: String,
    pub lead_provider: String,
    pub coder_provider: String,
    pub lead_agent_id: String,
    pub coder_agent_id: String,
    pub lead_online: bool,
    pub coder_online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lead_provider_session: Option<crate::daemon::types::ProviderConnectionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coder_provider_session: Option<crate::daemon::types::ProviderConnectionState>,
}
