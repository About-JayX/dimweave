use crate::daemon::{session_manager::SessionManager, types::BridgeMessage};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

pub type AgentSender = mpsc::Sender<BridgeMessage>;

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
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
            codex_inject_tx: None,
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            session_mgr: Arc::new(Mutex::new(SessionManager::new())),
        }
    }
}

impl DaemonState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn flush_buffered(&mut self) -> Vec<BridgeMessage> {
        std::mem::take(&mut self.buffered_messages)
    }

    pub fn buffer_message(&mut self, msg: BridgeMessage) {
        self.buffered_messages.push(msg);
        if self.buffered_messages.len() > 200 {
            self.buffered_messages.drain(0..100);
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
}
