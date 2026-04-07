use crate::daemon::types::{
    BridgeMessage, PermissionRequest, ProviderConnectionState, RuntimeHealthStatus,
};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_session: Option<ProviderConnectionState>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealthEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<RuntimeHealthStatus>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PermissionPromptEvent {
    pub agent: String,
    pub request_id: String,
    pub tool_name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_preview: Option<String>,
    pub created_at: u64,
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

/// Codex streaming event — thinking, deltas, and agent messages.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CodexStreamPayload {
    Thinking,
    Delta {
        text: String,
    },
    Message {
        text: String,
    },
    TurnDone {
        status: String,
    },
    /// Codex started a discrete work item (command, file edit, etc.)
    Activity {
        label: String,
    },
    /// Streaming reasoning summary text (accumulated)
    Reasoning {
        text: String,
    },
    /// Streaming command stdout/stderr
    CommandOutput {
        text: String,
    },
}

pub fn emit_codex_stream(app: &AppHandle, payload: CodexStreamPayload) {
    let _ = app.emit("codex_stream", payload);
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ClaudeStreamPayload {
    ThinkingStarted,
    Preview { text: String },
    Done,
    Reset,
}

#[cfg(test)]
fn should_auto_finish_idle_claude_thinking(_payload: &ClaudeStreamPayload) -> bool {
    false
}

pub fn emit_claude_stream(app: &AppHandle, payload: ClaudeStreamPayload) {
    let _ = app.emit("claude_stream", payload);
}

pub fn emit_agent_status(
    app: &AppHandle,
    agent: &str,
    online: bool,
    exit_code: Option<i32>,
    provider_session: Option<ProviderConnectionState>,
) {
    let _ = app.emit(
        "agent_status",
        AgentStatusEvent {
            agent: agent.into(),
            online,
            exit_code,
            provider_session,
        },
    );
}

pub fn emit_runtime_health(app: &AppHandle, health: Option<RuntimeHealthStatus>) {
    let _ = app.emit("runtime_health", RuntimeHealthEvent { health });
}

pub fn emit_permission_prompt(
    app: &AppHandle,
    agent: &str,
    request: &PermissionRequest,
    created_at: u64,
) {
    let _ = app.emit(
        "permission_prompt",
        PermissionPromptEvent {
            agent: agent.into(),
            request_id: request.request_id.clone(),
            tool_name: request.tool_name.clone(),
            description: request.description.clone(),
            input_preview: request.input_preview.clone(),
            created_at,
        },
    );
}

pub fn emit_telegram_state(app: &AppHandle, state: &crate::telegram::types::TelegramRuntimeState) {
    let _ = app.emit("telegram_state", state.clone());
}

#[cfg(test)]
mod tests {
    use super::{should_auto_finish_idle_claude_thinking, ClaudeStreamPayload};

    #[test]
    fn thinking_started_does_not_auto_finish_idle_claude_thinking() {
        assert!(!should_auto_finish_idle_claude_thinking(
            &ClaudeStreamPayload::ThinkingStarted
        ));
    }

    #[test]
    fn preview_does_not_auto_finish_idle_claude_thinking() {
        assert!(!should_auto_finish_idle_claude_thinking(
            &ClaudeStreamPayload::Preview {
                text: "preview".into(),
            }
        ));
    }
}
