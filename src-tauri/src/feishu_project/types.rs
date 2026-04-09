use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeishuSyncMode {
    #[default]
    Todo,
    Issues,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Unauthorized,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuProjectConfig {
    pub enabled: bool,
    // ── MCP fields (primary) ────────────────────────────────
    #[serde(default = "default_domain")]
    pub domain: String,
    #[serde(default)]
    pub mcp_user_token: String,
    #[serde(default)]
    pub workspace_hint: String,
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_minutes: u64,
    #[serde(default)]
    pub sync_mode: FeishuSyncMode,
    // ── DEPRECATED: legacy token/webhook fields ──────────────────
    // Kept only for serde backwards compat with persisted config.
    // Not used by the active MCP path. Do not add new references.
    #[serde(default)]
    pub project_key: String,
    #[serde(default)]
    pub plugin_token: String,
    #[serde(default)]
    pub user_key: String,
    #[serde(default)]
    pub webhook_token: String,
    #[serde(default)]
    pub poll_interval_minutes: u64,
    #[serde(default)]
    pub public_webhook_base_url: Option<String>,
    #[serde(default)]
    pub last_poll_at: Option<u64>,
    #[serde(default)]
    pub last_webhook_at: Option<u64>,
    #[serde(default)]
    pub last_sync_at: Option<u64>,
    #[serde(default)]
    pub last_error: Option<String>,
}

fn default_domain() -> String {
    "https://project.feishu.cn".into()
}

fn default_refresh_interval() -> u64 {
    10
}

impl Default for FeishuProjectConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domain: default_domain(),
            mcp_user_token: String::new(),
            workspace_hint: String::new(),
            refresh_interval_minutes: default_refresh_interval(),
            sync_mode: FeishuSyncMode::default(),
            project_key: String::new(),
            plugin_token: String::new(),
            user_key: String::new(),
            webhook_token: String::new(),
            poll_interval_minutes: 0,
            public_webhook_base_url: None,
            last_poll_at: None,
            last_webhook_at: None,
            last_sync_at: None,
            last_error: None,
        }
    }
}

/// Masked runtime state safe for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeishuProjectRuntimeState {
    pub enabled: bool,
    pub domain: Option<String>,
    pub workspace_hint: Option<String>,
    pub refresh_interval_minutes: u64,
    pub sync_mode: FeishuSyncMode,
    pub project_name: Option<String>,
    pub team_members: Vec<String>,
    pub mcp_status: McpConnectionStatus,
    pub discovered_tool_count: usize,
    pub last_sync_at: Option<u64>,
    pub last_error: Option<String>,
    pub token_label: Option<String>,
}

impl FeishuProjectRuntimeState {
    pub fn from_config(cfg: &FeishuProjectConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            domain: if cfg.domain.is_empty() {
                None
            } else {
                Some(cfg.domain.clone())
            },
            workspace_hint: if cfg.workspace_hint.is_empty() {
                None
            } else {
                Some(cfg.workspace_hint.clone())
            },
            refresh_interval_minutes: cfg.refresh_interval_minutes,
            sync_mode: cfg.sync_mode,
            project_name: None,
            team_members: Vec::new(),
            mcp_status: McpConnectionStatus::Disconnected,
            discovered_tool_count: 0,
            last_sync_at: cfg.last_sync_at,
            last_error: cfg.last_error.clone(),
            token_label: mask_token(&cfg.mcp_user_token),
        }
    }
}

fn mask_token(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let prefix_len = 5.min(token.len());
    Some(format!("{}***", &token[..prefix_len]))
}

/// A single Feishu Project work item in the Bug Inbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuProjectInboxItem {
    pub record_id: String,
    pub project_key: String,
    pub work_item_id: String,
    pub work_item_type_key: String,
    pub title: String,
    pub status_label: Option<String>,
    pub assignee_label: Option<String>,
    pub updated_at: u64,
    pub source_url: String,
    pub raw_snapshot_ref: String,
    pub ignored: bool,
    pub linked_task_id: Option<String>,
    pub last_ingress: IngressSource,
    pub last_event_uuid: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngressSource {
    Poll,
    Webhook,
    Mcp,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
