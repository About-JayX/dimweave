use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use crate::daemon::types::{BridgeMessage, MessageStatus};

/// Process a routed message and apply task state transitions.
pub fn process_message(store: &mut TaskGraphStore, task_id: &str, msg: &BridgeMessage) -> Vec<BridgeMessage> {
    let status = msg.status.unwrap_or(MessageStatus::Done);

    // lead → coder: start implementation (only if not already implementing+)
    if msg.source_role() == "lead" && msg.target_str() == "coder" {
        let current = store.get_task(task_id).map(|t| t.status);
        if matches!(current, Some(TaskStatus::Draft) | Some(TaskStatus::Planning)) {
            store.update_task_status(task_id, TaskStatus::Implementing);
        }
    }

    // coder → lead (done): lead reviews next
    if msg.source_role() == "coder" && msg.target_str() == "lead" && status.is_terminal() {
        store.update_task_status(task_id, TaskStatus::Reviewing);
        return Vec::new();
    }

    // lead → user (done): task complete
    if msg.source_role() == "lead" && msg.is_to_user() && status == MessageStatus::Done {
        store.update_task_status(task_id, TaskStatus::Done);
    }

    Vec::new()
}

/// Suggest the best routing target based on task state.
pub fn preferred_auto_target(task: &Task) -> Option<String> {
    if matches!(task.status, TaskStatus::Reviewing) {
        Some("lead".into())
    } else if matches!(task.status, TaskStatus::Implementing) {
        Some("coder".into())
    } else {
        Some("lead".into())
    }
}
