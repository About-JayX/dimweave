use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeishuProjectConfig {
    pub enabled: bool,
    pub project_key: String,
    pub plugin_token: String,
    pub user_key: String,
    pub webhook_token: String,
    pub poll_interval_minutes: u64,
    pub public_webhook_base_url: Option<String>,
    pub last_poll_at: Option<u64>,
    pub last_webhook_at: Option<u64>,
    pub last_sync_at: Option<u64>,
    pub last_error: Option<String>,
}

/// Masked runtime state safe for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeishuProjectRuntimeState {
    pub enabled: bool,
    pub project_key: Option<String>,
    pub token_label: Option<String>,
    pub user_key: Option<String>,
    pub poll_interval_minutes: u64,
    pub public_webhook_base_url: Option<String>,
    pub local_webhook_path: String,
    pub last_poll_at: Option<u64>,
    pub last_webhook_at: Option<u64>,
    pub last_sync_at: Option<u64>,
    pub last_error: Option<String>,
    pub webhook_enabled: bool,
}

impl FeishuProjectRuntimeState {
    pub fn from_config(cfg: &FeishuProjectConfig, local_webhook_path: &str) -> Self {
        Self {
            enabled: cfg.enabled,
            project_key: if cfg.project_key.is_empty() {
                None
            } else {
                Some(cfg.project_key.clone())
            },
            token_label: mask_plugin_token(&cfg.plugin_token),
            user_key: if cfg.user_key.is_empty() {
                None
            } else {
                Some(cfg.user_key.clone())
            },
            poll_interval_minutes: cfg.poll_interval_minutes,
            public_webhook_base_url: cfg.public_webhook_base_url.clone(),
            local_webhook_path: local_webhook_path.to_string(),
            last_poll_at: cfg.last_poll_at,
            last_webhook_at: cfg.last_webhook_at,
            last_sync_at: cfg.last_sync_at,
            last_error: cfg.last_error.clone(),
            webhook_enabled: cfg.public_webhook_base_url.is_some(),
        }
    }
}

pub fn mask_plugin_token(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let prefix_len = 5.min(token.len());
    let prefix = &token[..prefix_len];
    Some(format!("{prefix}***"))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_plugin_token_hides_secret_part() {
        assert_eq!(mask_plugin_token("plugin_secret_123"), Some("plugi***".into()));
    }

    #[test]
    fn mask_plugin_token_handles_empty() {
        assert_eq!(mask_plugin_token(""), None);
    }

    #[test]
    fn mask_plugin_token_handles_short() {
        assert_eq!(mask_plugin_token("abc"), Some("abc***".into()));
    }

    #[test]
    fn runtime_state_masks_plugin_token() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            project_key: "manciyuan".into(),
            plugin_token: "plugin_secret_123".into(),
            user_key: "u_123".into(),
            webhook_token: "hook_456".into(),
            poll_interval_minutes: 10,
            public_webhook_base_url: Some("https://abc.ngrok.app".into()),
            ..Default::default()
        };
        let state = FeishuProjectRuntimeState::from_config(
            &cfg,
            "/integrations/feishu-project/webhook",
        );
        assert_eq!(state.project_key.as_deref(), Some("manciyuan"));
        assert_eq!(state.token_label.as_deref(), Some("plugi***"));
        assert_eq!(state.user_key.as_deref(), Some("u_123"));
        assert_eq!(state.local_webhook_path, "/integrations/feishu-project/webhook");
        assert!(state.webhook_enabled);
    }

    #[test]
    fn runtime_state_without_public_url_disables_webhook() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            project_key: "proj".into(),
            plugin_token: "tok".into(),
            public_webhook_base_url: None,
            ..Default::default()
        };
        let state = FeishuProjectRuntimeState::from_config(&cfg, "/wh");
        assert!(!state.webhook_enabled);
    }

    #[test]
    fn runtime_state_empty_project_key_is_none() {
        let cfg = FeishuProjectConfig::default();
        let state = FeishuProjectRuntimeState::from_config(&cfg, "/wh");
        assert!(state.project_key.is_none());
        assert!(state.token_label.is_none());
        assert!(state.user_key.is_none());
    }
}
