use super::gui_task;
use super::routing;
use super::SharedState;
use crate::daemon::types::{Attachment, BridgeMessage};
use crate::feishu_project::types::FeishuProjectInboxItem;
use tauri::AppHandle;

/// Create or reuse a Dimweave task for a Bug Inbox work item.
pub async fn start_handling(
    state: &SharedState,
    app: &AppHandle,
    work_item_id: &str,
) -> Result<String, String> {
    // Look up work item and check for existing link
    let (item, existing_task_id) = {
        let daemon = state.read().await;
        let item = daemon
            .feishu_project_store
            .find_by_work_item_id(work_item_id)
            .cloned()
            .ok_or_else(|| format!("work item not found: {work_item_id}"))?;
        let tid = item.linked_task_id.clone();
        (item, tid)
    };

    // Reuse existing task if still present; stale link falls through to create
    if let Some(task_id) = existing_task_id {
        if state.write().await.select_task(&task_id).is_ok() {
            gui_task::emit_task_context_events(state, app, &task_id).await;
            return Ok(task_id);
        }
    }

    // Derive workspace from current context
    let workspace = resolve_workspace(state).await;
    let title = format!("[{}] {}", item.work_item_type_key, item.title);
    let task_id = {
        let mut daemon = state.write().await;
        let task = daemon.create_and_select_task(&workspace, &title);
        if let Some(it) = daemon
            .feishu_project_store
            .find_by_work_item_id_mut(work_item_id)
        {
            it.linked_task_id = Some(task.task_id.clone());
        }
        task.task_id
    };

    // Write snapshot and persist store
    let snapshot_path = write_snapshot(&item, &task_id)?;
    crate::feishu_project::runtime::persist_and_emit(state, app).await;

    // Route handoff message to lead through daemon routing
    let msg = build_handoff_message(&item, &task_id, &snapshot_path);
    routing::route_message(state, app, msg).await;

    gui_task::emit_task_context_events(state, app, &task_id).await;
    Ok(task_id)
}

/// Derive workspace from the currently active task or provider connection cwd.
async fn resolve_workspace(state: &SharedState) -> String {
    let s = state.read().await;
    // Prefer active task's workspace
    if let Some(ref tid) = s.active_task_id {
        if let Some(task) = s.task_graph.get_task(tid) {
            return task.workspace_root.clone();
        }
    }
    // Fall back to connected provider cwd
    if let Some(ref conn) = s.claude_connection {
        return conn.cwd.clone();
    }
    if let Some(ref conn) = s.codex_connection {
        return conn.cwd.clone();
    }
    ".".into()
}

fn snapshot_dir() -> Result<std::path::PathBuf, String> {
    let base = dirs::config_dir().ok_or("no config dir")?;
    Ok(base
        .join("com.dimweave.app")
        .join("feishu_project_snapshots"))
}

fn write_snapshot(item: &FeishuProjectInboxItem, task_id: &str) -> Result<String, String> {
    let dir = snapshot_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{task_id}.json"));
    let json = serde_json::to_string_pretty(item).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn build_handoff_message(
    item: &FeishuProjectInboxItem,
    task_id: &str,
    snapshot_path: &str,
) -> BridgeMessage {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    BridgeMessage {
        id: format!("fp_handoff_{task_id}_{now}"),
        from: "system".into(),
        display_source: Some("feishu_project".into()),
        to: "lead".into(),
        content: format!(
            "Feishu Project repair task created.\n\n\
             **[{}]** {}\n\
             Type: `{}` | Source: {}\n\
             Task: `{}`\n\n\
             Start by writing a repair plan from the attached snapshot, then follow \
             the plan \u{2192} execute \u{2192} review \u{2192} CM flow.",
            item.work_item_id,
            item.title,
            item.work_item_type_key,
            item.source_url,
            task_id,
        ),
        timestamp: now,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some(task_id.into()),
        session_id: None,
        sender_agent_id: None,
        attachments: Some(vec![Attachment {
            file_path: snapshot_path.into(),
            file_name: format!("{task_id}.json"),
            is_image: false,
            media_type: Some("application/json".into()),
        }]),
    }
}

#[cfg(test)]
#[path = "feishu_project_task_link_tests.rs"]
mod tests;
