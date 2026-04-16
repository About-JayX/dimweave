use super::*;
use crate::daemon::types::MessageTarget;

impl DaemonState {
    pub fn should_route_claude_sdk_text_directly(&self) -> bool {
        !self.attached_agents.contains_key("claude")
            || matches!(
                self.claude_sdk_direct_text_state,
                ClaudeSdkDirectTextState::Active
            )
    }

    pub fn prepare_claude_response_turn(&mut self) {
        self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
    }

    pub fn begin_claude_sdk_direct_text_turn(&mut self) -> bool {
        if matches!(
            self.claude_sdk_direct_text_state,
            ClaudeSdkDirectTextState::CompletedBySdk | ClaudeSdkDirectTextState::CompletedByBridge
        ) {
            self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
        }
        let should_route = self.should_route_claude_sdk_text_directly();
        if should_route {
            self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Active;
        }
        should_route
    }

    pub fn claim_claude_sdk_terminal_delivery(&mut self) -> bool {
        match self.claude_sdk_direct_text_state {
            ClaudeSdkDirectTextState::Active | ClaudeSdkDirectTextState::Inactive => {
                self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedBySdk;
                true
            }
            // Bridge already delivered this turn's result — suppress SDK duplicate
            ClaudeSdkDirectTextState::CompletedByBridge => false,
            // SDK already delivered — suppress duplicate
            ClaudeSdkDirectTextState::CompletedBySdk => false,
        }
    }

    pub fn claim_claude_bridge_terminal_delivery(&mut self) -> bool {
        match self.claude_sdk_direct_text_state {
            ClaudeSdkDirectTextState::Active | ClaudeSdkDirectTextState::Inactive => {
                self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedByBridge;
                true
            }
            ClaudeSdkDirectTextState::CompletedBySdk
            | ClaudeSdkDirectTextState::CompletedByBridge => false,
        }
    }

    pub fn finish_claude_sdk_direct_text_turn(&mut self) {
        self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
    }

    pub fn online_role_conflict(&self, _agent: &str, _role: &str) -> Option<&'static str> {
        // Per-agent-id routing supports same-role coexistence; no longer blocked.
        None
    }

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

    pub fn migrate_buffered_role(&mut self, old_role: &str, new_role: &str) {
        for msg in &mut self.buffered_messages {
            if msg.target_str() == old_role {
                msg.target = MessageTarget::Role { role: new_role.to_string() };
            }
        }
    }

    pub fn take_buffered_for(&mut self, role: &str) -> Vec<BridgeMessage> {
        self.take_buffered_for_task(role, None)
    }

    pub fn take_buffered_for_task(
        &mut self,
        role: &str,
        task_id: Option<&str>,
    ) -> Vec<BridgeMessage> {
        let mut ready = Vec::new();
        self.buffered_messages.retain(|msg| {
            let same_task = match (task_id, msg.task_id.as_deref()) {
                (Some(expected), Some(actual)) => expected == actual,
                (Some(_), None) => false,
                _ => true,
            };
            if msg.target_str() == role && same_task {
                ready.push(msg.clone());
                false
            } else {
                true
            }
        });
        ready
    }

    fn buffered_message_matches_task_scope(
        &self,
        message: &BridgeMessage,
        expected_task_id: Option<&str>,
    ) -> bool {
        // Phase 1: validate the task boundary itself. When we know which task a
        // message should belong to, reject mismatched task ids early.
        if let Some(task_id) = expected_task_id {
            if self.task_graph.get_task(task_id).is_none() {
                return false;
            }
            if let Some(message_task_id) = message.task_id.as_deref() {
                if message_task_id != task_id {
                    return false;
                }
            }
        }

        if let Some(task_id) = message.task_id.as_deref() {
            if self.task_graph.get_task(task_id).is_none() {
                return false;
            }
        }

        // Phase 2: if the message carries a session id, that session must still
        // exist and remain attached to the same task after hydration.
        if let Some(session_id) = message.session_id.as_deref() {
            let Some(session) = self.task_graph.get_session(session_id) else {
                return false;
            };
            if let Some(task_id) = message.task_id.as_deref() {
                if session.task_id != task_id {
                    return false;
                }
            }
            if let Some(expected) = expected_task_id {
                if session.task_id != expected {
                    return false;
                }
            }
        }

        true
    }

    pub(crate) fn persisted_buffered_messages(&self) -> Vec<BridgeMessage> {
        self.buffered_messages.clone()
    }

    pub(crate) fn restore_persisted_buffered_messages(&mut self, messages: Vec<BridgeMessage>) {
        let original_len = messages.len();
        self.buffered_messages = messages
            .into_iter()
            .filter(|message| self.buffered_message_matches_task_scope(message, None))
            .collect();
        let dropped = original_len.saturating_sub(self.buffered_messages.len());
        if dropped > 0 {
            eprintln!(
                "[Daemon] dropped {dropped} persisted buffered messages with invalid task/session bindings"
            );
        }
    }
}
