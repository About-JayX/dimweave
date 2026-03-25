use crate::daemon::{
    session_manager::SessionManager,
    types::{BridgeMessage, PermissionBehavior, PermissionRequest, PermissionVerdict, ToAgent},
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

pub const PERMISSION_TTL_MS: u64 = 10 * 60 * 1000;

pub type AgentSender = mpsc::Sender<ToAgent>;

struct PendingPermission {
    agent_id: String,
    created_at: u64,
    #[allow(dead_code)]
    request: PermissionRequest,
}

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pending_permissions: HashMap<String, PendingPermission>,
    buffered_verdicts: HashMap<String, Vec<PermissionVerdict>>,
    pub codex_inject_tx: Option<mpsc::Sender<String>>,
    pub claude_role: String,
    pub codex_role: String,
    /// Singleton session manager — shared across all Codex launches to avoid
    /// stale-session cleanup killing live sessions.
    pub session_mgr: Arc<Mutex<SessionManager>>,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            attached_agents: HashMap::new(),
            buffered_messages: Vec::new(),
            pending_permissions: HashMap::new(),
            buffered_verdicts: HashMap::new(),
            codex_inject_tx: None,
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            session_mgr: Arc::new(Mutex::new(SessionManager::new())),
        }
    }
}

impl DaemonState {
    pub fn new() -> Self { Self::default() }

    #[cfg(test)]
    pub fn flush_buffered(&mut self) -> Vec<BridgeMessage> {
        std::mem::take(&mut self.buffered_messages)
    }

    pub fn buffer_message(&mut self, msg: BridgeMessage) {
        self.buffered_messages.push(msg);
        if self.buffered_messages.len() > 200 {
            self.buffered_messages.drain(0..100);
            eprintln!("[Daemon] buffer overflow: 100 oldest messages dropped");
        }
    }

    pub fn take_buffered_for(&mut self, role: &str) -> Vec<BridgeMessage> {
        let mut ready = Vec::new();
        self.buffered_messages.retain(|msg| {
            if msg.to == role {
                ready.push(msg.clone());
                false
            } else {
                true
            }
        });
        ready
    }

    pub fn store_permission_request(
        &mut self,
        agent_id: &str,
        request: PermissionRequest,
        created_at: u64,
    ) {
        self.prune_expired_permissions(created_at);
        self.pending_permissions.insert(
            request.request_id.clone(),
            PendingPermission {
                agent_id: agent_id.to_string(),
                created_at,
                request,
            },
        );
    }

    pub fn resolve_permission(
        &mut self,
        request_id: &str,
        behavior: PermissionBehavior,
        now_ms: u64,
    ) -> Option<(String, ToAgent)> {
        self.prune_expired_permissions(now_ms);
        let pending = self.pending_permissions.remove(request_id)?;
        Some((
            pending.agent_id,
            ToAgent::PermissionVerdict {
                verdict: PermissionVerdict {
                    request_id: request_id.to_string(),
                    behavior,
                },
            },
        ))
    }

    pub fn buffer_permission_verdict(&mut self, agent_id: &str, verdict: PermissionVerdict) {
        let entry = self.buffered_verdicts.entry(agent_id.to_string()).or_default();
        entry.push(verdict);
        if entry.len() > 50 { entry.drain(0..25); }
    }

    pub fn take_buffered_verdicts_for(&mut self, agent_id: &str) -> Vec<PermissionVerdict> {
        self.buffered_verdicts.remove(agent_id).unwrap_or_default()
    }

    fn prune_expired_permissions(&mut self, now_ms: u64) {
        self.pending_permissions
            .retain(|_, pending| now_ms.saturating_sub(pending.created_at) <= PERMISSION_TTL_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_clears_buffer() {
        let mut s = DaemonState::new();
        s.buffer_message(BridgeMessage::system("hello", "lead"));
        assert_eq!(s.buffered_messages.len(), 1);
        let flushed = s.flush_buffered();
        assert_eq!(flushed.len(), 1);
        assert!(s.buffered_messages.is_empty());
    }

    #[test]
    fn buffer_caps_at_200() {
        let mut s = DaemonState::new();
        for i in 0..250 {
            s.buffer_message(BridgeMessage::system(&format!("msg{i}"), "lead"));
        }
        assert!(s.buffered_messages.len() <= 200);
    }

    #[test]
    fn permission_requests_round_trip_to_verdicts() {
        let mut s = DaemonState::new();
        s.store_permission_request(
            "claude",
            PermissionRequest {
                request_id: "req-1".into(),
                tool_name: "Bash".into(),
                description: "run ls".into(),
                input_preview: Some("ls".into()),
            },
            100,
        );

        let (agent_id, outbound) = s
            .resolve_permission("req-1", PermissionBehavior::Allow, 200)
            .expect("pending permission should resolve");

        assert_eq!(agent_id, "claude");
        match outbound {
            ToAgent::PermissionVerdict { verdict } => {
                assert_eq!(verdict.request_id, "req-1");
                assert!(matches!(verdict.behavior, PermissionBehavior::Allow));
            }
            other => panic!("unexpected outbound message: {other:?}"),
        }
    }

    #[test]
    fn expired_permissions_are_rejected() {
        let mut s = DaemonState::new();
        s.store_permission_request(
            "claude",
            PermissionRequest {
                request_id: "req-expired".into(),
                tool_name: "Bash".into(),
                description: "run rm".into(),
                input_preview: None,
            },
            100,
        );

        let result = s.resolve_permission(
            "req-expired", PermissionBehavior::Deny, 100 + PERMISSION_TTL_MS + 1,
        );
        assert!(result.is_none());
    }
}
