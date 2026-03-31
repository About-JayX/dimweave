use super::*;
use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::ReviewStatus;
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

impl DaemonState {
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
        let Some(task_id) = self.active_task_id.as_deref() else { return };
        let Some(task) = self.task_graph.get_task(task_id) else { return };
        message.task_id = Some(task.task_id.clone());
        message.session_id = match role {
            "lead" => task.lead_session_id.clone(),
            "coder" => task.current_coder_session_id.clone(),
            _ => None,
        };
    }

    pub fn prepare_task_routing(&mut self, msg: &BridgeMessage) -> TaskRoutingDecision {
        let Some(task_id) = self.active_task_id.clone() else {
            return TaskRoutingDecision { is_allowed: true, buffer_reason: None };
        };
        let Some(task) = self.task_graph.get_task(&task_id) else {
            return TaskRoutingDecision { is_allowed: true, buffer_reason: None };
        };
        if self.review_gate.should_block(task.review_status, msg) {
            self.review_gate.buffer_message(&task_id, msg.clone());
            return TaskRoutingDecision {
                is_allowed: false,
                buffer_reason: Some("review_gate"),
            };
        }
        TaskRoutingDecision { is_allowed: true, buffer_reason: None }
    }

    pub fn observe_task_message(&mut self, msg: &BridgeMessage) -> Vec<BridgeMessage> {
        let Some(task_id) = self.active_task_id.clone() else {
            return Vec::new();
        };
        let released = task_flow::process_message(
            &mut self.task_graph,
            &mut self.review_gate,
            &task_id,
            msg,
        );
        self.auto_save_task_graph();
        released
    }

    /// Lead explicitly approves the current review round.
    /// Only effective when review_status == PendingLeadApproval.
    pub fn lead_approve_review(&mut self) -> Vec<BridgeMessage> {
        let Some(task_id) = self.active_task_id.clone() else {
            return Vec::new();
        };
        let released = task_flow::lead_approve(
            &mut self.task_graph,
            &mut self.review_gate,
            &task_id,
        );
        self.auto_save_task_graph();
        released
    }
}
