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
    /// The agent instance that originated this message (e.g. "claude", "codex").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
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
    OnlineAgentsResponse {
        online_agents: serde_json::Value,
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
    GetOnlineAgents(tokio::sync::oneshot::Sender<serde_json::Value>),
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
    GetOnlineAgents,
    AgentDisconnect,
}

// ── Structured routing types (migration target) ─────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageSource {
    User,
    System,
    Agent {
        #[serde(rename = "agentId")]
        agent_id: String,
        role: String,
        provider: Provider,
        #[serde(rename = "displaySource", skip_serializing_if = "Option::is_none")]
        display_source: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageTarget {
    User,
    Role { role: String },
    Agent {
        #[serde(rename = "agentId")]
        agent_id: String,
    },
}

/// Migration-target message type with structured `source`/`target`/`reply_target`.
/// Coexists with legacy `BridgeMessage` until all producers/consumers migrate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectedBridgeMessage {
    pub id: String,
    pub source: MessageSource,
    pub target: MessageTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_target: Option<MessageTarget>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
}

#[cfg(test)]
mod directed_tests {
    use super::*;

    #[test]
    fn directed_msg_user_to_role() {
        let msg = DirectedBridgeMessage {
            id: "msg_1".into(),
            source: MessageSource::User,
            target: MessageTarget::Role { role: "coder".into() },
            reply_target: None,
            content: "Do this.".into(),
            timestamp: 1770000000000,
            reply_to: None,
            priority: None,
            status: None,
            task_id: Some("t1".into()),
            session_id: None,
            attachments: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["source"]["kind"], "user");
        assert_eq!(json["target"]["kind"], "role");
        assert_eq!(json["target"]["role"], "coder");
        assert_eq!(json["taskId"], "t1");
        assert!(json.get("replyTarget").is_none());
    }

    #[test]
    fn directed_msg_agent_to_agent_with_reply_target() {
        let msg = DirectedBridgeMessage {
            id: "msg_2".into(),
            source: MessageSource::Agent {
                agent_id: "lead_1".into(),
                role: "lead".into(),
                provider: Provider::Claude,
                display_source: Some("claude".into()),
            },
            target: MessageTarget::Agent { agent_id: "coder_2".into() },
            reply_target: Some(MessageTarget::Agent { agent_id: "lead_1".into() }),
            content: "Fix it.".into(),
            timestamp: 1770000000100,
            reply_to: None,
            priority: None,
            status: Some(MessageStatus::InProgress),
            task_id: None,
            session_id: None,
            attachments: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["source"]["kind"], "agent");
        assert_eq!(json["source"]["agentId"], "lead_1");
        assert_eq!(json["source"]["provider"], "claude");
        assert_eq!(json["source"]["displaySource"], "claude");
        assert_eq!(json["target"]["kind"], "agent");
        assert_eq!(json["target"]["agentId"], "coder_2");
        assert_eq!(json["replyTarget"]["kind"], "agent");
        assert_eq!(json["replyTarget"]["agentId"], "lead_1");
        assert_eq!(json["status"], "in_progress");
    }

    #[test]
    fn directed_msg_roundtrip() {
        let msg = DirectedBridgeMessage {
            id: "msg_3".into(),
            source: MessageSource::Agent {
                agent_id: "c1".into(),
                role: "coder".into(),
                provider: Provider::Codex,
                display_source: None,
            },
            target: MessageTarget::User,
            reply_target: None,
            content: "Done.".into(),
            timestamp: 1770000000200,
            reply_to: Some("msg_2".into()),
            priority: None,
            status: Some(MessageStatus::Done),
            task_id: None,
            session_id: None,
            attachments: None,
        };
        let json_str = serde_json::to_string(&msg).unwrap();
        let decoded: DirectedBridgeMessage = serde_json::from_str(&json_str).unwrap();
        assert_eq!(decoded.id, "msg_3");
        assert_eq!(decoded.source, MessageSource::Agent {
            agent_id: "c1".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: None,
        });
        assert_eq!(decoded.target, MessageTarget::User);
        assert_eq!(decoded.reply_target, None);
        assert_eq!(decoded.status, Some(MessageStatus::Done));
        assert_eq!(decoded.reply_to, Some("msg_2".into()));
    }
}
