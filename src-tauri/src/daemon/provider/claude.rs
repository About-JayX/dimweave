//! Claude provider adapter — maps Claude session lifecycle into the
//! normalized task graph session model and local transcript history.

use crate::daemon::provider::shared::{
    ProviderHistoryEntry, ProviderHistoryPage, ProviderResumeTarget, SessionRegistration,
};
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Register a new Claude session into the normalized task graph.
pub fn register_session(store: &mut TaskGraphStore, reg: SessionRegistration) -> SessionHandle {
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
    if let Some(transcript_path) = &reg.transcript_path {
        store.set_transcript_path(&sess.session_id, transcript_path);
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

/// Bind a Claude transcript path to an existing normalized session.
pub fn bind_transcript_path(
    store: &mut TaskGraphStore,
    session_id: &str,
    transcript_path: &str,
) -> bool {
    store.set_transcript_path(session_id, transcript_path)
}

/// Register a managed Claude launch into the normalized task graph.
pub fn register_on_launch(
    state: &mut crate::daemon::DaemonState,
    task_id: &str,
    role_id: &str,
    cwd: &str,
    claude_session_id: &str,
    transcript_path: &str,
) {
    let task_id = task_id.to_string();
    let session_role = match role_id {
        "coder" => SessionRole::Coder,
        _ => SessionRole::Lead,
    };
    let parent_id = if session_role == SessionRole::Coder {
        state
            .task_graph
            .get_task(&task_id)
            .and_then(|t| t.lead_session_id.clone())
    } else {
        None
    };

    let normalized_session_id = if let Some(existing) = state
        .task_graph
        .find_session_by_external_id(Provider::Claude, claude_session_id)
        .cloned()
    {
        let _ = state
            .task_graph
            .update_session_status(&existing.session_id, SessionStatus::Active);
        let _ = bind_transcript_path(&mut state.task_graph, &existing.session_id, transcript_path);
        existing.session_id
    } else {
        let reg = SessionRegistration {
            task_id: task_id.clone(),
            parent_session_id: parent_id,
            role: session_role,
            cwd: cwd.into(),
            title: format!("Claude {role_id}"),
            external_id: Some(claude_session_id.to_string()),
            transcript_path: Some(transcript_path.to_string()),
        };
        register_session(&mut state.task_graph, reg).session_id
    };

    match session_role {
        SessionRole::Lead => {
            state
                .task_graph
                .set_lead_session(&task_id, &normalized_session_id);
        }
        SessionRole::Coder => {
            state
                .task_graph
                .set_coder_session(&task_id, &normalized_session_id);
        }
    }
    state.auto_save_task_graph();
}

/// Called when Claude bridge connects: if there is an active task,
/// register a normalized session and update the task's lead or coder
/// pointer accordingly. No-op when no active task exists.
pub fn register_on_connect(
    state: &mut crate::daemon::DaemonState,
    role_id: &str,
    cwd: &str,
    claude_session_id: Option<&str>,
) {
    let Some(task_id) = state.active_task_id.clone() else {
        return;
    };
    let session_role = match role_id {
        "coder" => SessionRole::Coder,
        _ => SessionRole::Lead,
    };
    let parent_id = if session_role == SessionRole::Coder {
        state
            .task_graph
            .get_task(&task_id)
            .and_then(|t| t.lead_session_id.clone())
    } else {
        None
    };
    let reg = SessionRegistration {
        task_id: task_id.clone(),
        parent_session_id: parent_id,
        role: session_role,
        cwd: cwd.into(),
        title: format!("Claude {role_id}"),
        external_id: claude_session_id.map(ToOwned::to_owned),
        transcript_path: claude_session_id
            .map(|session_id| default_transcript_path(cwd, session_id))
            .transpose()
            .ok()
            .flatten()
            .map(|path| path.to_string_lossy().to_string()),
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

pub fn workspace_history_dir(workspace_root: &str, claude_projects_root: &Path) -> PathBuf {
    let normalized = normalize_workspace_root(workspace_root);
    let normalized = normalized.to_string_lossy();
    let mut slug = String::with_capacity(normalized.len());
    for ch in normalized.chars() {
        match ch {
            '/' | '\\' | ':' | '_' => slug.push('-'),
            _ => slug.push(ch),
        }
    }
    claude_projects_root.join(slug)
}

pub fn transcript_path_for(
    workspace_root: &str,
    session_id: &str,
    claude_projects_root: &Path,
) -> PathBuf {
    workspace_history_dir(workspace_root, claude_projects_root).join(format!("{session_id}.jsonl"))
}

pub fn default_projects_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".claude").join("projects"))
        .ok_or_else(|| "failed to resolve Claude projects root".to_string())
}

pub fn default_transcript_path(workspace_root: &str, session_id: &str) -> Result<PathBuf, String> {
    Ok(transcript_path_for(
        workspace_root,
        session_id,
        &default_projects_root()?,
    ))
}

pub fn list_sessions(
    workspace_root: &str,
    claude_projects_root: Option<&Path>,
) -> Result<ProviderHistoryPage, String> {
    let root = match claude_projects_root {
        Some(root) => root.to_path_buf(),
        None => default_projects_root()?,
    };
    let history_dir = workspace_history_dir(workspace_root, &root);
    if !history_dir.exists() {
        return Ok(ProviderHistoryPage {
            entries: Vec::new(),
            next_cursor: None,
        });
    }

    let mut entries = Vec::new();
    let dir = std::fs::read_dir(&history_dir).map_err(|err| {
        format!(
            "failed to read Claude history dir {}: {err}",
            history_dir.display()
        )
    })?;
    for entry in dir {
        let entry = entry.map_err(|err| {
            format!(
                "failed to iterate Claude history dir {}: {err}",
                history_dir.display()
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(history_entry) = summarize_transcript(&path)? {
            entries.push(history_entry);
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

pub fn build_resume_target(session: &SessionHandle) -> Result<ProviderResumeTarget, String> {
    if session.provider != Provider::Claude {
        return Err(format!(
            "session {} is not a Claude session",
            session.session_id
        ));
    }
    let external_id = session.external_session_id.clone().ok_or_else(|| {
        format!(
            "session {} missing external Claude session id",
            session.session_id
        )
    })?;
    Ok(ProviderResumeTarget {
        role: session.role,
        cwd: session.cwd.clone(),
        external_id,
    })
}

fn summarize_transcript(path: &Path) -> Result<Option<ProviderHistoryEntry>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read Claude transcript {}: {err}", path.display()))?;
    let mut session_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string);
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
        if session_id.is_none() {
            session_id = value["sessionId"].as_str().map(ToOwned::to_owned);
        }
        if cwd.is_none() {
            cwd = value["cwd"].as_str().map(ToOwned::to_owned);
        }
        if let Some(ts) = parse_timestamp(&value) {
            created_at = Some(created_at.map_or(ts, |existing: u64| existing.min(ts)));
            updated_at = Some(updated_at.map_or(ts, |existing: u64| existing.max(ts)));
        }
        match value["message"]["role"].as_str() {
            Some("user") if title.is_none() => {
                title = extract_message_text(&value["message"]["content"]);
            }
            Some("assistant") => {
                if let Some(text) = extract_message_text(&value["message"]["content"]) {
                    preview = Some(text);
                }
            }
            _ => {}
        }
    }

    let Some(external_id) = session_id else {
        return Ok(None);
    };
    Ok(Some(ProviderHistoryEntry {
        provider: Provider::Claude,
        external_id,
        title,
        preview,
        cwd,
        archived: false,
        created_at: created_at.unwrap_or_default(),
        updated_at: updated_at.unwrap_or_default(),
        status: SessionStatus::Paused,
        normalized_session_id: None,
        normalized_task_id: None,
    }))
}

fn parse_timestamp(value: &Value) -> Option<u64> {
    let timestamp = value["timestamp"].as_str()?;
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
    Some(parsed.timestamp_millis() as u64)
}

fn extract_message_text(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(text) => Some(text.as_str()),
        Value::Array(items) => items.iter().find_map(|item| {
            if item["type"].as_str() == Some("text") {
                item["text"].as_str()
            } else {
                None
            }
        }),
        _ => None,
    }?;
    normalize_text(text)
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
        strip_channel_wrapper(&collapsed).or(Some(collapsed))
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
