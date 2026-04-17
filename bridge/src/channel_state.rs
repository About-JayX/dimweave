use crate::types::{BridgeMessage, PermissionRequest, PermissionVerdict};
use std::collections::HashMap;

/// Minimal channel state: sender validation + pending permission tracking.
/// Routing is handled by the daemon, not the bridge.
#[derive(Default)]
pub struct ChannelState {
    pending_permissions: HashMap<String, PermissionRequest>,
}

impl ChannelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert a daemon-routed BridgeMessage into a Claude channel notification.
    /// Returns None only if the sender field is empty (the daemon is the routing
    /// authority, so arbitrary role names are accepted).
    pub fn prepare_channel_message(&self, msg: &BridgeMessage) -> Option<serde_json::Value> {
        if msg.source_role().trim().is_empty() {
            eprintln!("[Bridge/channel] dropped message {} with empty sender", msg.id);
            return None;
        }

        Some(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/claude/channel",
            "params": {
                "message": msg.message,
                "meta": build_meta(msg)
            }
        }))
    }

    pub fn register_permission(&mut self, request: PermissionRequest) {
        self.pending_permissions
            .insert(request.request_id.clone(), request);
    }

    pub fn permission_notification(
        &mut self,
        verdict: PermissionVerdict,
    ) -> Option<serde_json::Value> {
        self.pending_permissions.remove(&verdict.request_id)?;
        Some(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/claude/channel/permission",
            "params": {
                "request_id": verdict.request_id,
                "behavior": verdict.behavior,
            }
        }))
    }
}

fn build_meta(msg: &BridgeMessage) -> serde_json::Value {
    let mut meta = serde_json::Map::new();
    meta.insert("from".into(), serde_json::json!(msg.source_role()));
    if let Some(status) = msg.status {
        meta.insert("status".into(), serde_json::json!(status.as_str()));
    }
    if let Some(agent_id) = msg.source_agent_id() {
        meta.insert("sender_agent_id".into(), serde_json::json!(agent_id));
    }
    serde_json::Value::Object(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BridgeMessage, MessageSource, MessageTarget, Provider};

    fn msg(from: &str) -> BridgeMessage {
        let source = match from {
            "user" => MessageSource::User,
            "system" => MessageSource::System,
            _ => MessageSource::Agent {
                agent_id: from.into(),
                role: from.into(),
                provider: Provider::Claude,
                display_source: None,
            },
        };
        BridgeMessage {
            id: "msg-1".into(),
            source,
            target: MessageTarget::Role { role: "lead".into() },
            reply_target: None,
            message: "hello".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: None,
            attachments: None,
        }
    }

    #[test]
    fn allowed_sender_produces_notification() {
        let state = ChannelState::new();
        let notif = state.prepare_channel_message(&msg("coder"));
        assert!(notif.is_some());
        assert_eq!(notif.unwrap()["params"]["meta"]["from"], "coder");
    }

    #[test]
    fn status_is_forwarded_in_channel_meta() {
        let state = ChannelState::new();
        let mut message = msg("coder");
        message.status = Some(crate::types::MessageStatus::InProgress);
        let notif = state.prepare_channel_message(&message).unwrap();
        assert_eq!(notif["params"]["meta"]["status"], "in_progress");
    }

    #[test]
    fn sender_agent_id_is_forwarded_in_channel_meta() {
        let state = ChannelState::new();
        // The `msg("coder")` helper creates a MessageSource::Agent with agent_id="coder"
        let notif = state.prepare_channel_message(&msg("coder")).unwrap();
        assert_eq!(notif["params"]["meta"]["sender_agent_id"], "coder");
    }

    #[test]
    fn sender_agent_id_absent_when_source_is_user() {
        let state = ChannelState::new();
        let notif = state.prepare_channel_message(&msg("user")).unwrap();
        assert!(notif["params"]["meta"]["sender_agent_id"].is_null());
    }

    #[test]
    fn arbitrary_role_sender_is_accepted() {
        let state = ChannelState::new();
        let notif = state.prepare_channel_message(&msg("reviewer"));
        assert!(notif.is_some());
        assert_eq!(notif.unwrap()["params"]["meta"]["from"], "reviewer");
    }

    #[test]
    fn empty_sender_is_dropped() {
        let state = ChannelState::new();
        assert!(state.prepare_channel_message(&msg("")).is_none());
        assert!(state.prepare_channel_message(&msg("  ")).is_none());
    }
}
