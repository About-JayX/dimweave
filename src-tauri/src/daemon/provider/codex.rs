//! Codex provider adapter — maps Codex thread lifecycle into the
//! normalized task graph session model.

use crate::daemon::provider::shared::SessionRegistration;
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;

/// Register a new Codex session into the normalized task graph.
/// If `reg.external_id` is set, it is bound immediately (typical for
/// sessions where the thread_id is known at creation time).
pub fn register_session(store: &mut TaskGraphStore, reg: SessionRegistration) -> SessionHandle {
    let sess = store.create_session(CreateSessionParams {
        task_id: &reg.task_id,
        parent_session_id: reg.parent_session_id.as_deref(),
        provider: Provider::Codex,
        role: reg.role,
        cwd: &reg.cwd,
        title: &reg.title,
    });
    if let Some(ext_id) = &reg.external_id {
        store.set_external_session_id(&sess.session_id, ext_id);
    }
    // Re-fetch to return the updated record
    store.get_session(&sess.session_id).cloned().unwrap_or(sess)
}

/// Bind a Codex thread_id to an existing session.
/// Use when the thread_id becomes available after session creation
/// (e.g. after Codex handshake completes).
pub fn bind_thread_id(store: &mut TaskGraphStore, session_id: &str, thread_id: &str) -> bool {
    store.set_external_session_id(session_id, thread_id)
}

/// Called at Codex launch time: if there is an active task, register
/// a normalized coder session with the given thread_id and update the
/// task's current_coder_session_id.  No-op when no active task exists.
pub fn register_on_launch(
    state: &mut crate::daemon::DaemonState,
    role_id: &str,
    cwd: &str,
    thread_id: &str,
) {
    let Some(task_id) = state.active_task_id.clone() else {
        return;
    };
    let parent_id = state
        .task_graph
        .get_task(&task_id)
        .and_then(|t| t.lead_session_id.clone());
    let session_role = match role_id {
        "lead" => SessionRole::Lead,
        _ => SessionRole::Coder,
    };
    let reg = SessionRegistration {
        task_id: task_id.clone(),
        parent_session_id: parent_id,
        role: session_role,
        cwd: cwd.into(),
        title: format!("Codex {role_id}"),
        external_id: Some(thread_id.into()),
    };
    let sess = register_session(&mut state.task_graph, reg);
    if session_role == SessionRole::Coder {
        state
            .task_graph
            .set_coder_session(&task_id, &sess.session_id);
    }
    state.auto_save_task_graph();
}

// ── Future adapter entry points (not yet implemented) ──────
//
// pub fn list_threads(...)   — thread/list via Codex WS
// pub fn resume_thread(...)  — thread/resume
// pub fn fork_thread(...)    — thread/fork
// pub fn archive_thread(...) — thread/archive
