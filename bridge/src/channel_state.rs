use crate::types::{BridgeMessage, PermissionRequest, PermissionVerdict};
use std::collections::HashMap;

const ALLOWED_SENDERS: &[&str] = &["user", "system", "lead", "coder"];

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
    /// Returns None if the sender is not in the allow list.
    pub fn prepare_channel_message(&self, msg: &BridgeMessage) -> Option<serde_json::Value> {
        if !ALLOWED_SENDERS.contains(&msg.from.as_str()) {
            eprintln!(
                "[Bridge/channel] dropped message {} from unknown sender {}",
                msg.id, msg.from
            );
            return None;
        }

        Some(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/claude/channel",
            "params": {
                "content": msg.content,
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
    meta.insert("from".into(), serde_json::json!(msg.from));
    if let Some(status) = msg.status {
        meta.insert("status".into(), serde_json::json!(status.as_str()));
    }
    if let Some(ref agent_id) = msg.sender_agent_id {
        meta.insert("sender_agent_id".into(), serde_json::json!(agent_id));
    }
    serde_json::Value::Object(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BridgeMessage;

    fn msg(from: &str) -> BridgeMessage {
        BridgeMessage {
            id: "msg-1".into(),
            from: from.into(),
            display_source: None,
            to: "lead".into(),
            content: "hello".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: None,
            sender_agent_id: None,
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
        let mut message = msg("coder");
        message.sender_agent_id = Some("codex".into());
        let notif = state.prepare_channel_message(&message).unwrap();
        assert_eq!(notif["params"]["meta"]["sender_agent_id"], "codex");
    }

    #[test]
    fn sender_agent_id_absent_when_none() {
        let state = ChannelState::new();
        let notif = state.prepare_channel_message(&msg("coder")).unwrap();
        assert!(notif["params"]["meta"]["sender_agent_id"].is_null());
    }

    #[test]
    fn unknown_sender_is_dropped() {
        let state = ChannelState::new();
        assert!(state.prepare_channel_message(&msg("intruder")).is_none());
    }

    #[test]
    fn removed_role_sender_is_dropped() {
        let state = ChannelState::new();
        assert!(state.prepare_channel_message(&msg("tester")).is_none());
    }
}
