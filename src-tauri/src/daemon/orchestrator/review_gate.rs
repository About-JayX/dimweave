use crate::daemon::task_graph::types::ReviewStatus;
use crate::daemon::types::BridgeMessage;
use std::collections::HashMap;

/// Manages the review gate: blocks lead→coder messages during
/// active review and releases them only on explicit lead approval.
pub struct ReviewGate {
    blocked: HashMap<String, Vec<BridgeMessage>>,
}

impl ReviewGate {
    pub fn new() -> Self {
        Self {
            blocked: HashMap::new(),
        }
    }

    /// Should this message be blocked by the review gate?
    /// Blocks lead→coder whenever any review status is active.
    pub fn should_block(&self, review_status: Option<ReviewStatus>, msg: &BridgeMessage) -> bool {
        msg.from == "lead" && msg.to == "coder" && review_status.is_some()
    }

    /// Buffer a message that was blocked by the gate.
    pub fn buffer_message(&mut self, task_id: &str, msg: BridgeMessage) {
        self.blocked
            .entry(task_id.to_string())
            .or_default()
            .push(msg);
    }

    /// Lead explicitly approves the current review round.
    /// Returns any buffered messages that should now be released.
    pub fn approve(&mut self, task_id: &str) -> Vec<BridgeMessage> {
        self.blocked.remove(task_id).unwrap_or_default()
    }

    /// Drain blocked messages for a task (e.g. on task completion).
    pub fn drain(&mut self, task_id: &str) {
        self.blocked.remove(task_id);
    }
}

impl Default for ReviewGate {
    fn default() -> Self {
        Self::new()
    }
}
