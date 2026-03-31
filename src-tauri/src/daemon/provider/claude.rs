//! Claude provider adapter — maps Claude session lifecycle into the
//! normalized task graph session model.

use crate::daemon::provider::shared::SessionRegistration;
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;

/// Register a new Claude session into the normalized task graph.
pub fn register_session(
    store: &mut TaskGraphStore,
    reg: SessionRegistration,
) -> SessionHandle {
    let sess = store.create_session(CreateSessionParams {
        task_id: &reg.task_id,
        parent_session_id: reg.parent_session_id.as_deref(),
        provider: Provider::Claude,
        role: reg.role,
        cwd: &reg.cwd,
        title: &reg.title,
    });
    if let Some(ext_id) = &reg.external_id {
        store.set_external_session_id(&sess.session_id, ext_id);
    }
    store.get_session(&sess.session_id).cloned().unwrap_or(sess)
}

/// Bind a Claude session_id to an existing normalized session.
pub fn bind_session_id(
    store: &mut TaskGraphStore,
    session_id: &str,
    claude_session_id: &str,
) -> bool {
    store.set_external_session_id(session_id, claude_session_id)
}

/// Called when Claude bridge connects: if there is an active task,
/// register a normalized session and update the task's lead or coder
/// pointer accordingly.  No-op when no active task exists.
pub fn register_on_connect(
    state: &mut crate::daemon::DaemonState,
    role_id: &str,
    cwd: &str,
    claude_session_id: Option<&str>,
) {
    let Some(task_id) = state.active_task_id.clone() else { return };
    let session_role = match role_id {
        "coder" => SessionRole::Coder,
        _ => SessionRole::Lead,
    };
    let parent_id = if session_role == SessionRole::Coder {
        state.task_graph.get_task(&task_id).and_then(|t| t.lead_session_id.clone())
    } else {
        None
    };
    let reg = SessionRegistration {
        task_id: task_id.clone(),
        parent_session_id: parent_id,
        role: session_role,
        cwd: cwd.into(),
        title: format!("Claude {role_id}"),
        external_id: claude_session_id.map(String::from),
    };
    let sess = register_session(&mut state.task_graph, reg);
    match session_role {
        SessionRole::Lead => { state.task_graph.set_lead_session(&task_id, &sess.session_id); }
        SessionRole::Coder => { state.task_graph.set_coder_session(&task_id, &sess.session_id); }
    }
    state.auto_save_task_graph();
}

// ── Future adapter entry points (not yet implemented) ──────
//
// pub fn list_sessions(...)   — local transcript/history index
// pub fn resume_session(...)  — reconnect to a Claude session
// pub fn capture_metadata(...) — extract session metadata from PTY
