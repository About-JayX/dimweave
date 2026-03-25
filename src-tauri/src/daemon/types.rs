use serde::{Deserialize, Serialize};

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
}

impl BridgeMessage {
    pub fn system(content: &str, to: &str) -> Self {
        Self {
            id: format!("sys_{}", chrono::Utc::now().timestamp_millis()),
            from: "system".into(),
            to: to.into(),
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reply_to: None,
            priority: None,
        }
    }
}

/// daemon → bridge (over WS :4502)
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToAgent {
    RoutedMessage { message: BridgeMessage },
}

/// bridge → daemon (over WS :4502)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromAgent {
    AgentConnect {
        #[serde(rename = "agentId")]
        agent_id: String,
    },
    AgentReply {
        message: BridgeMessage,
    },
    AgentDisconnect,
}
