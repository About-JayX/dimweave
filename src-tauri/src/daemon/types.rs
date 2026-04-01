use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    InProgress,
    Done,
    Error,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "in_progress" => Some(Self::InProgress),
            "done" => Some(Self::Done),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Error)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderConnectionMode {
    New,
    Resumed,
}

impl ProviderConnectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Resumed => "resumed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionState {
    pub provider: crate::daemon::task_graph::types::Provider,
    pub external_session_id: String,
    pub cwd: String,
    pub connection_mode: ProviderConnectionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_source: Option<String>,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<MessageStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The agent instance that originated this message (e.g. "claude", "codex").
    /// Set by the daemon on inbound AgentReply; distinct from `from` (which is the role).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_agent_id: Option<String>,
}

impl BridgeMessage {
    #[cfg(test)]
    pub fn system(content: &str, to: &str) -> Self {
        Self {
            id: format!("sys_{}", chrono::Utc::now().timestamp_millis()),
            from: "system".into(),
            display_source: Some("system".into()),
            to: to.into(),
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reply_to: None,
            priority: None,
            status: None,
            task_id: None,
            session_id: None,
            sender_agent_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeStatus {
    pub agent: String,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_session: Option<ProviderConnectionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatusSnapshot {
    pub agents: Vec<AgentRuntimeStatus>,
    pub claude_role: String,
    pub codex_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PermissionVerdict {
    pub request_id: String,
    pub behavior: PermissionBehavior,
}

/// Frontend DTO: active task with its sessions and artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub task: crate::daemon::task_graph::types::Task,
    pub sessions: Vec<crate::daemon::task_graph::types::SessionHandle>,
    pub artifacts: Vec<crate::daemon::task_graph::types::Artifact>,
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

/// daemon → bridge (over WS :4502)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToAgent {
    RoutedMessage { message: BridgeMessage },
    PermissionVerdict { verdict: PermissionVerdict },
    OnlineAgentsResponse { online_agents: serde_json::Value },
}

/// bridge → daemon (over WS :4502)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromAgent {
    // Note: `agentId` uses camelCase (not snake_case) for wire compatibility
    // with the bridge. Both sides use `#[serde(rename = "agentId")]`.
    AgentConnect {
        #[serde(rename = "agentId")]
        agent_id: String,
    },
    AgentReply {
        message: BridgeMessage,
    },
    PermissionRequest {
        request: PermissionRequest,
    },
    GetOnlineAgents,
    AgentDisconnect,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
