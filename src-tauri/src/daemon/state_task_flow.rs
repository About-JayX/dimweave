use super::*;
use crate::daemon::gui_task::{build_task_change_events, TaskUiEvent};
use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::{Provider, ReviewStatus, SessionHandle};
use crate::daemon::types::BridgeMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRoutingDecision {
    pub is_allowed: bool,
    pub buffer_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveReviewGate {
    pub task_id: String,
    pub review_status: ReviewStatus,
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
        if let Some(session_id) = message.session_id.as_deref() {
            return self.task_graph.get_session(session_id);
        }

        let task_id = message.task_id.as_deref()?;
        self.task_session_for_role(task_id, &message.to)
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

    pub fn set_active_task(&mut self, task_id: Option<String>) {
        self.active_task_id = task_id;
    }

    pub fn active_review_gate(&self) -> Option<ActiveReviewGate> {
        let task_id = self.active_task_id.as_ref()?;
        let task = self.task_graph.get_task(task_id)?;
        let review_status = task.review_status?;
        Some(ActiveReviewGate {
            task_id: task.task_id.clone(),
            review_status,
        })
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
        let Some(task_id) = self.active_task_id.clone() else {
            return TaskRoutingDecision {
                is_allowed: true,
                buffer_reason: None,
            };
        };
        let Some(task) = self.task_graph.get_task(&task_id) else {
            return TaskRoutingDecision {
                is_allowed: true,
                buffer_reason: None,
            };
        };
        if self.review_gate.should_block(task.review_status, msg) {
            self.review_gate.buffer_message(&task_id, msg.clone());
            return TaskRoutingDecision {
                is_allowed: false,
                buffer_reason: Some("review_gate"),
            };
        }
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
        let released =
            task_flow::process_message(&mut self.task_graph, &mut self.review_gate, &task_id, msg);
        self.auto_save_task_graph();
        let after = self.task_graph.get_task(&task_id).cloned();
        TaskFlowEffect {
            released,
            ui_events: build_task_change_events(before.as_ref(), after.as_ref()),
        }
    }

    /// Lead explicitly approves the current review round.
    /// Only effective when review_status == PendingLeadApproval.
    pub fn lead_approve_review(&mut self) -> Vec<BridgeMessage> {
        self.lead_approve_review_effects().released
    }

    pub fn lead_approve_review_effects(&mut self) -> TaskFlowEffect {
        let Some(task_id) = self.active_task_id.clone() else {
            return TaskFlowEffect::default();
        };
        let before = self.task_graph.get_task(&task_id).cloned();
        let released =
            task_flow::lead_approve(&mut self.task_graph, &mut self.review_gate, &task_id);
        self.auto_save_task_graph();
        let after = self.task_graph.get_task(&task_id).cloned();
        TaskFlowEffect {
            released,
            ui_events: build_task_change_events(before.as_ref(), after.as_ref()),
        }
    }
}
