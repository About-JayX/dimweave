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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<MessageStatus>,
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

/// Messages daemon sends TO bridge over WS :4502
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonMsg {
    RoutedMessage {
        message: BridgeMessage,
    },
    PermissionVerdict {
        verdict: PermissionVerdict,
    },
    Status {
        #[serde(rename = "status")]
        _status: serde_json::Value,
    },
}

#[derive(Debug)]
pub enum DaemonInbound {
    RoutedMessage(BridgeMessage),
    PermissionVerdict(PermissionVerdict),
}

#[derive(Debug)]
pub enum BridgeOutbound {
    AgentReply(BridgeMessage),
    PermissionRequest(PermissionRequest),
}

/// Messages bridge sends TO daemon over WS :4502
#[allow(dead_code)]
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeMsg<'a> {
    // Note: `agentId` uses camelCase (not snake_case) for wire compatibility
    // with the daemon. Both sides use `#[serde(rename = "agentId")]`.
    AgentConnect {
        #[serde(rename = "agentId")]
        agent_id: &'a str,
    },
    AgentReply {
        message: &'a BridgeMessage,
    },
    PermissionRequest {
        request: &'a PermissionRequest,
    },
    AgentDisconnect,
}
