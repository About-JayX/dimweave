use crate::daemon::{
    task_graph::{
        store::TaskGraphStore,
        types::{SessionHandle, SessionRole},
    },
    types::BridgeMessage,
};

fn is_task_role(role: &str) -> bool {
    matches!(role, "lead" | "coder")
}

fn session_role_name(role: SessionRole) -> &'static str {
    match role {
        SessionRole::Lead => "lead",
        SessionRole::Coder => "coder",
    }
}

pub fn target_role_session<'a>(
    task_graph: &'a TaskGraphStore,
    task_id: &str,
    to_role: &str,
) -> Option<&'a SessionHandle> {
    let task = task_graph.get_task(task_id)?;
    let session_id = match to_role {
        "lead" => task.lead_session_id.as_deref()?,
        "coder" => task.current_coder_session_id.as_deref()?,
        _ => return None,
    };
    task_graph.get_session(session_id)
}

pub fn resolve_target_bound_session<'a>(
    task_graph: &'a TaskGraphStore,
    message: &BridgeMessage,
) -> Option<&'a SessionHandle> {
    if !is_task_role(message.target_str()) {
        return None;
    }

    if let Some(session_id) = message.session_id.as_deref() {
        let session = task_graph.get_session(session_id)?;
        let stamped_role = session_role_name(session.role);
        let stamped_matches_task = message
            .task_id
            .as_deref()
            .is_none_or(|task_id| session.task_id == task_id);

        if stamped_role == message.target_str() && stamped_matches_task {
            return Some(session);
        }
    }

    let task_id = message.task_id.as_deref()?;
    target_role_session(task_graph, task_id, message.target_str())
}
