//! Codex provider adapter — maps Codex thread lifecycle into the
//! normalized task graph session model.

use crate::daemon::provider::shared::{
    ProviderHistoryEntry, ProviderHistoryPage, ProviderResumeTarget, SessionRegistration,
};
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use serde_json::Value;
use std::path::{Path, PathBuf};
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
    if let Some(transcript_path) = &reg.transcript_path {
        store.set_transcript_path(&sess.session_id, transcript_path);
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
        transcript_path: None,
    };
    let sess = register_session(&mut state.task_graph, reg);
    match session_role {
        SessionRole::Lead => {
            state
                .task_graph
                .set_lead_session(&task_id, &sess.session_id);
        }
        SessionRole::Coder => {
            state
                .task_graph
                .set_coder_session(&task_id, &sess.session_id);
        }
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
            normalized_session_id: None,
            normalized_task_id: None,
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
        transcript_path: None,
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
    let result =
        crate::daemon::codex::ws_client::thread_list(port, Value::Object(rpc_params), app).await?;
    map_thread_page_response(&result, params.archived)
}

pub fn list_local_sessions(
    workspace_root: &str,
    sessions_root: Option<&Path>,
) -> Result<ProviderHistoryPage, String> {
    let root = match sessions_root {
        Some(root) => root.to_path_buf(),
        None => default_sessions_root()?,
    };
    if !root.exists() {
        return Ok(ProviderHistoryPage {
            entries: Vec::new(),
            next_cursor: None,
        });
    }

    let workspace_root = normalize_workspace_root(workspace_root);
    let mut files = Vec::new();
    collect_jsonl_files(&root, &mut files)?;

    let mut entries = Vec::new();
    for path in files {
        if let Some(entry) = summarize_local_session(&path, &workspace_root)? {
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.external_id.cmp(&b.external_id))
    });

    Ok(ProviderHistoryPage {
        entries,
        next_cursor: None,
    })
}

pub async fn fork_thread(port: u16, thread_id: &str, app: &AppHandle) -> Result<String, String> {
    crate::daemon::codex::ws_client::thread_fork(port, thread_id, app).await
}

pub async fn archive_thread(port: u16, thread_id: &str, app: &AppHandle) -> Result<(), String> {
    crate::daemon::codex::ws_client::thread_archive(port, thread_id, app).await
}

fn default_sessions_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".codex").join("sessions"))
        .ok_or_else(|| "failed to resolve Codex sessions root".to_string())
}

fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|err| format!("failed to read Codex history dir {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!(
                "failed to iterate Codex history dir {}: {err}",
                dir.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
    Ok(())
}

fn summarize_local_session(
    path: &Path,
    workspace_root: &Path,
) -> Result<Option<ProviderHistoryEntry>, String> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read Codex history file {}: {err}",
            path.display()
        )
    })?;

    let mut external_id = None;
    let mut cwd = None;
    let mut title = None;
    let mut preview = None;
    let mut created_at = None;
    let mut updated_at = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if let Some(ts) = parse_timestamp(&value["timestamp"]) {
            created_at = Some(created_at.map_or(ts, |existing: u64| existing.min(ts)));
            updated_at = Some(updated_at.map_or(ts, |existing: u64| existing.max(ts)));
        }

        match value["type"].as_str() {
            Some("session_meta") => {
                if external_id.is_none() {
                    external_id = value["payload"]["id"].as_str().map(ToOwned::to_owned);
                }
                if cwd.is_none() {
                    cwd = value["payload"]["cwd"].as_str().map(ToOwned::to_owned);
                }
                if let Some(ts) = parse_timestamp(&value["payload"]["timestamp"]) {
                    created_at = Some(created_at.map_or(ts, |existing: u64| existing.min(ts)));
                    updated_at = Some(updated_at.map_or(ts, |existing: u64| existing.max(ts)));
                }
            }
            Some("response_item") => {
                if title.is_none() {
                    title = extract_response_item_text(&value["payload"], "user");
                }
                if preview.is_none() {
                    preview = extract_response_item_text(&value["payload"], "assistant");
                }
            }
            Some("event_msg") => {
                if title.is_none() && value["payload"]["type"].as_str() == Some("user_message") {
                    title =
                        normalize_text(value["payload"]["message"].as_str().unwrap_or_default());
                }
                if value["payload"]["type"].as_str() == Some("agent_message") {
                    preview =
                        normalize_text(value["payload"]["message"].as_str().unwrap_or_default());
                }
            }
            Some("turn_context") if cwd.is_none() => {
                cwd = value["payload"]["cwd"].as_str().map(ToOwned::to_owned);
            }
            _ => {}
        }
    }

    let Some(external_id) = external_id else {
        return Ok(None);
    };
    let Some(session_cwd) = cwd else {
        return Ok(None);
    };
    if normalize_workspace_root(&session_cwd) != workspace_root {
        return Ok(None);
    }

    Ok(Some(ProviderHistoryEntry {
        provider: Provider::Codex,
        external_id,
        title,
        preview,
        cwd: Some(session_cwd),
        archived: false,
        created_at: created_at.unwrap_or_default(),
        updated_at: updated_at.unwrap_or_default(),
        status: SessionStatus::Paused,
        normalized_session_id: None,
        normalized_task_id: None,
    }))
}

fn extract_response_item_text(payload: &Value, role: &str) -> Option<String> {
    if payload["type"].as_str() != Some("message") || payload["role"].as_str() != Some(role) {
        return None;
    }
    let content = payload["content"].as_array()?;
    let text = content
        .iter()
        .find_map(|item| match item["type"].as_str() {
            Some("input_text") | Some("output_text") | Some("text") => item["text"].as_str(),
            _ => None,
        })?;
    normalize_text(text)
}

fn parse_timestamp(value: &Value) -> Option<u64> {
    let timestamp = value.as_str()?;
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
    Some(parsed.timestamp_millis() as u64)
}

fn normalize_text(text: &str) -> Option<String> {
    let collapsed = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.is_empty() {
        None
    } else {
        decode_structured_message(&collapsed)
            .or_else(|| strip_channel_wrapper(&collapsed))
            .and_then(|normalized| discard_meta_text(&normalized))
            .or_else(|| discard_meta_text(&collapsed))
    }
}

fn decode_structured_message(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let message = value["message"].as_str()?;
    let trimmed = message.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn strip_channel_wrapper(text: &str) -> Option<String> {
    if !text.starts_with("<channel") || !text.ends_with("</channel>") {
        return None;
    }
    let start = text.find('>')? + 1;
    let end = text.rfind("</channel>")?;
    let inner = text[start..end].trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

fn discard_meta_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("<environment_context>")
        || trimmed.starts_with("<permissions instructions>")
        || trimmed.starts_with("<skills_instructions>")
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_workspace_root(workspace_root: &str) -> PathBuf {
    let path = Path::new(workspace_root);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    absolute.canonicalize().unwrap_or(absolute)
}
