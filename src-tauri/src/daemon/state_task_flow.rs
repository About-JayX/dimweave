use super::*;
use crate::daemon::gui_task::{build_task_change_events, TaskUiEvent};
use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::{Provider, SessionHandle};
use crate::daemon::types::BridgeMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRoutingDecision {
    pub is_allowed: bool,
    pub buffer_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskFlowEffect {
    pub released: Vec<BridgeMessage>,
    pub ui_events: Vec<TaskUiEvent>,
}

impl DaemonState {
    fn task_session_for_role<'a>(&'a self, task_id: &str, role: &str) -> Option<&'a SessionHandle> {
        let task = self.task_graph.get_task(task_id)?;
        let session_id = match role {
            "lead" => task.lead_session_id.as_deref()?,
            "coder" => task.current_coder_session_id.as_deref()?,
            _ => return None,
        };
        self.task_graph.get_session(session_id)
    }

    fn bound_session_for_message<'a>(
        &'a self,
        message: &BridgeMessage,
    ) -> Option<&'a SessionHandle> {
        crate::daemon::routing_target_session::resolve_target_bound_session(
            &self.task_graph,
            message,
        )
    }

    fn agent_matches_bound_session(&self, agent: &str, session: &SessionHandle) -> bool {
        let expected_provider = match agent {
            "claude" => Provider::Claude,
            "codex" => Provider::Codex,
            _ => return false,
        };
        if session.provider != expected_provider {
            return false;
        }

        match self.provider_connection(agent) {
            Some(connection) => {
                session.external_session_id.as_deref()
                    == Some(connection.external_session_id.as_str())
            }
            None => session.external_session_id.is_none(),
        }
    }

    pub fn role_has_compatible_online_agent(&self, role: &str) -> bool {
        if role != "lead" && role != "coder" {
            return (self.is_agent_online("claude") && self.claude_role == role)
                || (self.is_agent_online("codex") && self.codex_role == role);
        }

        let Some(task_id) = self.active_task_id.as_deref() else {
            return (self.is_agent_online("claude") && self.claude_role == role)
                || (self.is_agent_online("codex") && self.codex_role == role);
        };
        let Some(session) = self.task_session_for_role(task_id, role) else {
            return false;
        };

        (self.is_agent_online("claude")
            && self.claude_role == role
            && self.agent_matches_bound_session("claude", session))
            || (self.is_agent_online("codex")
                && self.codex_role == role
                && self.agent_matches_bound_session("codex", session))
    }

    pub fn agent_matches_task_message(&self, agent: &str, message: &BridgeMessage) -> bool {
        if message.to != "lead" && message.to != "coder" {
            return true;
        }

        // Legacy/system messages without stamped task context still rely on
        // plain role routing until every caller carries task/session ownership.
        if message.session_id.is_none() && message.task_id.is_none() {
            return true;
        }

        let Some(session) = self.bound_session_for_message(message) else {
            return false;
        };

        self.agent_matches_bound_session(agent, session)
    }

    pub fn route_buffer_reason(&self, message: &BridgeMessage) -> Option<&'static str> {
        if message.to != "lead" && message.to != "coder" {
            return Some("target_agent_offline");
        }

        if self.bound_session_for_message(message).is_none() {
            return Some("target_session_missing");
        }

        let claude_online_mismatch =
            self.claude_role == message.to
                && self.is_agent_online("claude")
                && !self.agent_matches_task_message("claude", message);
        let codex_online_mismatch =
            self.codex_role == message.to
                && self.is_agent_online("codex")
                && !self.agent_matches_task_message("codex", message);

        if claude_online_mismatch || codex_online_mismatch {
            return Some("task_session_mismatch");
        }

        Some("target_agent_offline")
    }

    pub fn set_active_task(&mut self, task_id: Option<String>) {
        self.active_task_id = task_id;
    }

    pub fn preferred_auto_target(&self) -> Option<String> {
        let task = self
            .active_task_id
            .as_deref()
            .and_then(|id| self.task_graph.get_task(id))?;
        task_flow::preferred_auto_target(task)
    }

    pub fn stamp_message_context(&self, role: &str, message: &mut BridgeMessage) {
        let Some(task_id) = self.active_task_id.as_deref() else {
            return;
        };
        let Some(task) = self.task_graph.get_task(task_id) else {
            return;
        };
        message.task_id = Some(task.task_id.clone());
        message.session_id = match role {
            "lead" => task.lead_session_id.clone(),
            "coder" => task.current_coder_session_id.clone(),
            _ => None,
        };
    }

    pub fn prepare_task_routing(&mut self, msg: &BridgeMessage) -> TaskRoutingDecision {
        let _ = msg;
        TaskRoutingDecision {
            is_allowed: true,
            buffer_reason: None,
        }
    }

    pub fn observe_task_message(&mut self, msg: &BridgeMessage) -> Vec<BridgeMessage> {
        self.observe_task_message_effects(msg).released
    }

    pub fn observe_task_message_effects(&mut self, msg: &BridgeMessage) -> TaskFlowEffect {
        let Some(task_id) = self.active_task_id.clone() else {
            return TaskFlowEffect::default();
        };
        let before = self.task_graph.get_task(&task_id).cloned();
        let released = task_flow::process_message(&mut self.task_graph, &task_id, msg);
        self.auto_save_task_graph();
        let after = self.task_graph.get_task(&task_id).cloned();
        TaskFlowEffect {
            released,
            ui_events: build_task_change_events(before.as_ref(), after.as_ref()),
        }
    }
}
