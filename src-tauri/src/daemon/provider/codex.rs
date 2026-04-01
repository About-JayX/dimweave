//! Codex provider adapter — maps Codex thread lifecycle into the
//! normalized task graph session model.

use crate::daemon::provider::shared::{
    ProviderHistoryEntry, ProviderHistoryPage, ProviderResumeTarget, SessionRegistration,
};
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use serde_json::Value;
use tauri::AppHandle;

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

pub fn map_thread_page_response(
    response: &Value,
    archived: bool,
) -> Result<ProviderHistoryPage, String> {
    let data = response["data"]
        .as_array()
        .ok_or_else(|| "invalid thread/list response: missing data array".to_string())?;
    let mut entries = Vec::with_capacity(data.len());
    for item in data {
        let external_id = item["id"]
            .as_str()
            .ok_or_else(|| "invalid thread/list response: missing thread id".to_string())?;
        let status = match item["status"]["type"].as_str().unwrap_or("notLoaded") {
            "active" => SessionStatus::Active,
            "systemError" => SessionStatus::Error,
            _ => SessionStatus::Paused,
        };
        entries.push(ProviderHistoryEntry {
            provider: Provider::Codex,
            external_id: external_id.to_string(),
            title: item["name"].as_str().map(ToOwned::to_owned),
            preview: item["preview"].as_str().map(ToOwned::to_owned),
            cwd: item["cwd"].as_str().map(ToOwned::to_owned),
            archived,
            created_at: item["createdAt"].as_u64().unwrap_or_default(),
            updated_at: item["updatedAt"].as_u64().unwrap_or_default(),
            status,
        });
    }
    Ok(ProviderHistoryPage {
        entries,
        next_cursor: response["nextCursor"].as_str().map(ToOwned::to_owned),
    })
}

pub fn register_forked_session(
    store: &mut TaskGraphStore,
    source_session_id: &str,
    thread_id: &str,
    title: Option<&str>,
) -> Result<SessionHandle, String> {
    let source = store
        .get_session(source_session_id)
        .cloned()
        .ok_or_else(|| format!("session not found: {source_session_id}"))?;
    let parent_session_id = source
        .parent_session_id
        .clone()
        .or_else(|| Some(source.session_id.clone()));
    let reg = SessionRegistration {
        task_id: source.task_id.clone(),
        parent_session_id,
        role: source.role,
        cwd: source.cwd.clone(),
        title: title.unwrap_or(&source.title).to_string(),
        external_id: Some(thread_id.to_string()),
    };
    Ok(register_session(store, reg))
}

pub fn build_resume_target(session: &SessionHandle) -> Result<ProviderResumeTarget, String> {
    if session.provider != Provider::Codex {
        return Err(format!(
            "session {} is not a codex session",
            session.session_id
        ));
    }
    let external_id = session
        .external_session_id
        .clone()
        .ok_or_else(|| format!("session {} missing external thread id", session.session_id))?;
    Ok(ProviderResumeTarget {
        role: session.role,
        cwd: session.cwd.clone(),
        external_id,
    })
}

pub fn mark_session_archived(store: &mut TaskGraphStore, session_id: &str) -> bool {
    store.update_session_status(session_id, SessionStatus::Completed)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListThreadsParams {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub archived: bool,
    pub cwd: Option<String>,
}

pub async fn list_threads(
    port: u16,
    params: &ListThreadsParams,
    app: &AppHandle,
) -> Result<ProviderHistoryPage, String> {
    let mut rpc_params = serde_json::Map::new();
    if let Some(cursor) = params.cursor.as_deref() {
        rpc_params.insert("cursor".into(), Value::String(cursor.to_string()));
    }
    if let Some(limit) = params.limit {
        rpc_params.insert("limit".into(), Value::from(limit));
    }
    rpc_params.insert("sortKey".into(), Value::String("updated_at".into()));
    if params.archived {
        rpc_params.insert("archived".into(), Value::Bool(true));
    }
    if let Some(cwd) = params.cwd.as_deref() {
        rpc_params.insert("cwd".into(), Value::String(cwd.to_string()));
    }
    let result = crate::daemon::codex::ws_client::thread_list(port, Value::Object(rpc_params), app)
        .await?;
    map_thread_page_response(&result, params.archived)
}

pub async fn fork_thread(port: u16, thread_id: &str, app: &AppHandle) -> Result<String, String> {
    crate::daemon::codex::ws_client::thread_fork(port, thread_id, app).await
}

pub async fn archive_thread(port: u16, thread_id: &str, app: &AppHandle) -> Result<(), String> {
    crate::daemon::codex::ws_client::thread_archive(port, thread_id, app).await
}
