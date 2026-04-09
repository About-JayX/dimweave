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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
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
    /// Thinking block delta (extended thinking text)
    ThinkingDelta { text: String },
    /// Text block started
    TextStarted,
    /// Text block delta (assistant output text)
    TextDelta { text: String },
    /// Tool use block started
    ToolStarted { name: String },
    /// Legacy: accumulated preview text (kept for batching compat)
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
            role: None,
        },
    );
}

pub fn emit_agent_status_online(
    app: &AppHandle,
    agent: &str,
    provider_session: Option<ProviderConnectionState>,
    role: String,
) {
    let _ = app.emit(
        "agent_status",
        AgentStatusEvent {
            agent: agent.into(),
            online: true,
            exit_code: None,
            provider_session,
            role: Some(role),
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

pub fn emit_feishu_project_state(
    app: &AppHandle,
    state: &crate::feishu_project::types::FeishuProjectRuntimeState,
) {
    let _ = app.emit("feishu_project_state", state.clone());
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskSaveStatusEvent {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub task_id: String,
    pub timestamp: u64,
}

pub fn emit_task_save_status(app: &AppHandle, success: bool, error: Option<String>, task_id: &str) {
    let _ = app.emit(
        "task_save_status",
        TaskSaveStatusEvent {
            success,
            error,
            task_id: task_id.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        },
    );
}

pub fn emit_feishu_project_items(
    app: &AppHandle,
    items: &[crate::feishu_project::types::FeishuProjectInboxItem],
) {
    let _ = app.emit("feishu_project_items", items.to_vec());
}

#[cfg(test)]
#[path = "gui_tests.rs"]
mod tests;
