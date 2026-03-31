use crate::daemon::orchestrator::review_gate::ReviewGate;
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use crate::daemon::types::{BridgeMessage, MessageStatus};

/// Process a routed message and apply task state transitions.
/// Returns any messages released by gate changes.
pub fn process_message(
    store: &mut TaskGraphStore,
    gate: &mut ReviewGate,
    task_id: &str,
    msg: &BridgeMessage,
) -> Vec<BridgeMessage> {
    let status = msg.status.unwrap_or(MessageStatus::Done);

    // coder → lead (done): enter review phase
    if msg.from == "coder" && msg.to == "lead" && status.is_terminal() {
        store.update_task_status(task_id, TaskStatus::Reviewing);
        store.update_task_review_status(
            task_id,
            Some(ReviewStatus::PendingLeadReview),
        );
        return Vec::new();
    }

    // lead → reviewer: hand off to reviewer
    if msg.from == "lead" && msg.to == "reviewer" {
        store.update_task_status(task_id, TaskStatus::Reviewing);
        store.update_task_review_status(task_id, Some(ReviewStatus::InReview));
        return Vec::new();
    }

    // reviewer → coder: reviewer sends fixes back
    if msg.from == "reviewer" && msg.to == "coder" {
        store.update_task_status(task_id, TaskStatus::Implementing);
        store.update_task_review_status(
            task_id,
            Some(ReviewStatus::PendingLeadReview),
        );
        return Vec::new();
    }

    // reviewer → lead (done): reviewer finished, await lead approval
    if msg.from == "reviewer" && msg.to == "lead" && status == MessageStatus::Done {
        store.update_task_review_status(
            task_id,
            Some(ReviewStatus::PendingLeadApproval),
        );
        // Do NOT release blocked messages — lead must explicitly approve.
        return Vec::new();
    }

    // lead → user (done): task complete
    if msg.from == "lead" && msg.to == "user" && status == MessageStatus::Done {
        store.update_task_status(task_id, TaskStatus::Done);
        store.update_task_review_status(task_id, None);
        gate.drain(task_id);
    }

    Vec::new()
}

/// Lead explicitly approves the current review.
/// Only effective when review_status == PendingLeadApproval.
/// Clears the gate and returns released blocked messages.
pub fn lead_approve(
    store: &mut TaskGraphStore,
    gate: &mut ReviewGate,
    task_id: &str,
) -> Vec<BridgeMessage> {
    let Some(task) = store.get_task(task_id) else {
        return Vec::new();
    };
    if task.review_status != Some(ReviewStatus::PendingLeadApproval) {
        return Vec::new();
    }
    store.update_task_status(task_id, TaskStatus::Implementing);
    store.update_task_review_status(task_id, None);
    gate.approve(task_id)
}

/// Suggest the best routing target based on task state.
pub fn preferred_auto_target(task: &Task) -> Option<String> {
    if task.review_status.is_some()
        || matches!(task.status, TaskStatus::Reviewing)
    {
        Some("lead".into())
    } else if matches!(task.status, TaskStatus::Implementing) {
        Some("coder".into())
    } else {
        Some("lead".into())
    }
}
