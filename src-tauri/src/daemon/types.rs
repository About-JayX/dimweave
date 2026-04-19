use serde::{Deserialize, Serialize};

// Re-export frontend DTOs so existing `use daemon::types::X` paths keep working.
pub use super::types_dto::{
    HistoryEntry, OnlineAgentInfo, SessionTreeSnapshot, TaskProviderSummary, TaskSnapshot,
};
#[path = "types_runtime.rs"]
mod types_runtime;
pub use types_runtime::{RuntimeHealthLevel, RuntimeHealthStatus};

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
pub struct Attachment {
    pub file_path: String,
    pub file_name: String,
    #[serde(default)]
    pub is_image: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub source: MessageSource,
    pub target: MessageTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_target: Option<MessageTarget>,
    // `alias = "content"` keeps legacy persisted JSON blobs readable (the
    // `buffered_messages` table in `task_graph.sqlite` may have been
    // written under the pre-rename schema). Serialization always emits
    // `message`; deserialization accepts either. One-way ingest shim.
    #[serde(alias = "content")]
    pub message: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
}

impl BridgeMessage {
    pub fn is_from_user(&self) -> bool { matches!(self.source, MessageSource::User) }
    pub fn is_from_system(&self) -> bool { matches!(self.source, MessageSource::System) }

    pub fn source_role(&self) -> &str {
        match &self.source {
            MessageSource::User => "user",
            MessageSource::System => "system",
            MessageSource::Agent { role, .. } => role,
        }
    }

    pub fn target_str(&self) -> &str {
        match &self.target {
            MessageTarget::User => "user",
            MessageTarget::Role { role } => role,
            MessageTarget::Agent { agent_id } => agent_id,
        }
    }

    pub fn is_to_user(&self) -> bool { matches!(self.target, MessageTarget::User) }

    pub fn source_agent_id(&self) -> Option<&str> {
        match &self.source {
            MessageSource::Agent { agent_id, .. } => Some(agent_id),
            _ => None,
        }
    }

    pub fn source_display(&self) -> Option<&str> {
        match &self.source {
            MessageSource::Agent { display_source, .. } => display_source.as_deref(),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn system(content: &str, to: &str) -> Self {
        Self {
            id: format!("sys_{}", chrono::Utc::now().timestamp_millis()),
            source: MessageSource::System,
            target: if to == "user" {
                MessageTarget::User
            } else {
                MessageTarget::Role { role: to.into() }
            },
            reply_target: None,
            message: content.into(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reply_to: None,
            priority: None,
            status: None,
            task_id: None,
            session_id: None,
            attachments: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_health: Option<RuntimeHealthStatus>,
    /// Compatibility-only: reflects the global role label, not task-scoped ownership.
    /// Frontend should prefer `TaskProviderSummary` for per-task role→provider mapping.
    pub claude_role: String,
    /// Compatibility-only: reflects the global role label, not task-scoped ownership.
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
        /// Task this bridge instance belongs to. Absent for legacy clients
        /// that predate per-task handshake; daemon falls back to scanning
        /// task_runtimes (racy with multi-task) in that case.
        #[serde(rename = "taskId", default)]
        task_id: Option<String>,
        /// Concrete TaskAgent id. Used for AgentReply stamping so
        /// multi-task deployments don't collapse to one window.
        #[serde(rename = "taskAgentId", default)]
        task_agent_id: Option<String>,
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

// ── Structured routing types (migration target) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageSource {
    User,
    System,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: String,
        role: String,
        provider: crate::daemon::task_graph::types::Provider,
        #[serde(rename = "displaySource", skip_serializing_if = "Option::is_none")]
        display_source: Option<String>,
    },
}

pub use super::message_target::MessageTarget;

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
