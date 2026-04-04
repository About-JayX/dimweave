use super::*;

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
            ClaudeSdkDirectTextState::Active => {
                self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedBySdk;
                true
            }
            ClaudeSdkDirectTextState::Inactive => {
                if self.attached_agents.contains_key("claude") {
                    false
                } else {
                    self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedBySdk;
                    true
                }
            }
            ClaudeSdkDirectTextState::CompletedBySdk
            | ClaudeSdkDirectTextState::CompletedByBridge => false,
        }
    }

    pub fn claim_claude_bridge_terminal_delivery(&mut self) -> bool {
        match self.claude_sdk_direct_text_state {
            ClaudeSdkDirectTextState::Active => {
                self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedByBridge;
                true
            }
            ClaudeSdkDirectTextState::Inactive => true,
            ClaudeSdkDirectTextState::CompletedBySdk
            | ClaudeSdkDirectTextState::CompletedByBridge => false,
        }
    }

    pub fn finish_claude_sdk_direct_text_turn(&mut self) {
        self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::Inactive;
    }

    pub fn online_role_conflict(&self, agent: &str, role: &str) -> Option<&'static str> {
        match agent {
            "claude" if self.is_agent_online("codex") && self.codex_role == role => Some("codex"),
            "codex" if self.is_agent_online("claude") && self.claude_role == role => Some("claude"),
            _ => None,
        }
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
            if msg.to == old_role {
                msg.to = new_role.to_string();
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
            if msg.to == role && same_task {
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

    pub(crate) fn restore_review_gate_snapshot(
        &mut self,
        mut snapshot: crate::daemon::orchestrator::review_gate::ReviewGateSnapshot,
    ) {
        let original_tasks = snapshot.blocked.len();
        let original_messages: usize = snapshot.blocked.values().map(|messages| messages.len()).sum();
        snapshot.blocked.retain(|task_id, messages| {
            if self.task_graph.get_task(task_id).is_none() {
                return false;
            }
            // Inner retain prunes invalid persisted messages before deciding
            // whether the task-level review gate entry should survive restore.
            messages.retain(|message| {
                self.buffered_message_matches_task_scope(message, Some(task_id.as_str()))
            });
            !messages.is_empty()
        });
        let restored_messages: usize = snapshot.blocked.values().map(|messages| messages.len()).sum();
        let dropped_messages = original_messages.saturating_sub(restored_messages);
        let dropped_tasks = original_tasks.saturating_sub(snapshot.blocked.len());
        if dropped_messages > 0 || dropped_tasks > 0 {
            eprintln!(
                "[Daemon] dropped {dropped_messages} persisted review-gate messages across {dropped_tasks} invalid task buckets"
            );
        }
        self.review_gate.restore(snapshot);
    }
}
