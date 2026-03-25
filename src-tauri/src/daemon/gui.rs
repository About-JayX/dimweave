use crate::daemon::types::BridgeMessage;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageEvent {
    pub payload: BridgeMessage,
    pub timestamp: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemLogEvent {
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusEvent {
    pub agent: String,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

pub fn emit_agent_message(app: &AppHandle, msg: &BridgeMessage) {
    let _ = app.emit(
        "agent_message",
        AgentMessageEvent {
            payload: msg.clone(),
            timestamp: msg.timestamp,
        },
    );
}

pub fn emit_system_log(app: &AppHandle, level: &str, message: &str) {
    let _ = app.emit(
        "system_log",
        SystemLogEvent {
            level: level.into(),
            message: message.into(),
        },
    );
}

pub fn emit_agent_status(app: &AppHandle, agent: &str, online: bool, exit_code: Option<i32>) {
    let _ = app.emit(
        "agent_status",
        AgentStatusEvent {
            agent: agent.into(),
            online,
            exit_code,
        },
    );
}
