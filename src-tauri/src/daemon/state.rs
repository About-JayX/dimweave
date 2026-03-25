use crate::daemon::types::BridgeMessage;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub type AgentSender = mpsc::Sender<BridgeMessage>;

pub struct DaemonState {
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pub claude_role: String,
    pub codex_role: String,
    pub codex_bootstrapped: bool,
    pub active_thread_id: Option<String>,
    pub codex_home: Option<String>,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            attached_agents: HashMap::new(),
            buffered_messages: Vec::new(),
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            codex_bootstrapped: false,
            active_thread_id: None,
            codex_home: None,
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
        // Prevent unbounded growth
        if self.buffered_messages.len() > 200 {
            self.buffered_messages.drain(0..100);
        }
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
