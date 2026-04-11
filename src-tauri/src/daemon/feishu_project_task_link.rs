use super::gui::{self};
use super::gui_task;
use super::routing;
use super::SharedState;
use crate::daemon::types::{Attachment, BridgeMessage};
use crate::feishu_project::types::FeishuProjectInboxItem;
use serde_json::{json, Value};
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

    // Fetch full context from MCP (detail + comments)
    let context = fetch_issue_context(&item, app).await;

    // Derive workspace from current context
    let workspace = resolve_workspace(state).await;
    let title = format!("[{}] {}", item.work_item_type_key, item.title);
    let task_id = {
        let mut daemon = state.write().await;
        let task = daemon.create_and_select_task(&workspace, &title);
        let tid = task.task_id.clone();
        if let Some(it) = daemon
            .feishu_project_store
            .find_by_work_item_id_mut(work_item_id)
        {
            it.linked_task_id = Some(tid.clone());
        }
        if let Some(vi) = daemon.feishu_issue_view.iter_mut()
            .find(|i| i.work_item_id == work_item_id)
        {
            vi.linked_task_id = Some(tid.clone());
        }
        tid
    };

    // Write enriched snapshot and persist store
    let snapshot_path = write_context_snapshot(&context, &task_id)?;
    crate::feishu_project::runtime::persist_and_emit(state, app).await;

    // Route handoff message to lead through daemon routing
    let msg = build_handoff_message(&item, &context, &task_id, &snapshot_path);
    routing::route_message(state, app, msg).await;

    gui_task::emit_task_context_events(state, app, &task_id).await;
    Ok(task_id)
}

/// Fetch issue detail + comments from MCP, return structured context.
async fn fetch_issue_context(item: &FeishuProjectInboxItem, app: &AppHandle) -> Value {
    let cfg = crate::feishu_project::config::default_config_path()
        .and_then(|p| crate::feishu_project::config::load_config(&p))
        .unwrap_or_default();

    let mut context = json!({
        "work_item_id": item.work_item_id,
        "name": item.title,
        "type": item.work_item_type_key,
        "priority": item.status_label,
        "assignee": item.assignee_label,
        "source_url": item.source_url,
    });

    if cfg.mcp_user_token.is_empty() {
        return context;
    }

    let mut client =
        crate::feishu_project::mcp_client::McpClient::new(&cfg.domain, &cfg.mcp_user_token);
    if client.connect_lite().await.is_err() {
        return context;
    }

    // Fetch detail
    let detail_args = json!({
        "project_key": item.project_key,
        "work_item_id": item.work_item_id,
        "fields": ["description", "priority", "bug_classification", "issue_stage"]
    });
    if let Ok(result) = client.call_tool("get_workitem_brief", detail_args).await {
        if let Some(text) = first_text(&result) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                // Extract attributes
                let attr = &parsed["work_item_attribute"];
                if let Some(status) = attr["work_item_status"]["name"].as_str() {
                    context["status"] = json!(status);
                }
                if let Some(t) = attr["create_time"].as_str() {
                    context["created_at"] = json!(t);
                }
                if let Some(t) = attr["update_time"].as_str() {
                    context["updated_at"] = json!(t);
                }
                if let Some(reporter) = attr["role_members"]
                    .as_array()
                    .and_then(|arr| arr.iter().find(|r| r["key"] == "reporter"))
                {
                    context["reporter"] = reporter["members"].clone();
                }
                if let Some(operator) = attr["role_members"]
                    .as_array()
                    .and_then(|arr| arr.iter().find(|r| r["key"] == "operator"))
                {
                    context["operator"] = operator["members"].clone();
                }
                // Extract fields
                if let Some(fields) = parsed["work_item_fields"].as_array() {
                    for f in fields {
                        let key = f["key"].as_str().unwrap_or("");
                        match key {
                            "description" => {
                                context["description"] = f["value"].clone();
                            }
                            "priority" => {
                                context["priority"] =
                                    json!(f["value"]["label"].as_str().unwrap_or(""));
                            }
                            "bug_classification" => {
                                context["classification"] =
                                    json!(f["value"]["label"].as_str().unwrap_or(""));
                            }
                            "issue_stage" => {
                                context["stage"] =
                                    json!(f["value"]["label"].as_str().unwrap_or(""));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Fetch comments
    let comment_args = json!({
        "project_key": item.project_key,
        "work_item_id": item.work_item_id,
        "page_num": 1
    });
    if let Ok(result) = client.call_tool("list_workitem_comments", comment_args).await {
        if let Some(text) = first_text(&result) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                if let Some(comments) = parsed["comments"].as_array() {
                    let mapped: Vec<Value> = comments
                        .iter()
                        .map(|c| {
                            json!({
                                "author": c["creator"].as_str().unwrap_or(""),
                                "content": c["content"].as_str().unwrap_or(""),
                                "created_at": c["created_at"].as_str().unwrap_or(""),
                            })
                        })
                        .collect();
                    context["comments"] = json!(mapped);
                }
            }
        }
    }

    gui::emit_system_log(
        app,
        "info",
        &format!(
            "[FeishuProject] fetched context for {}",
            item.work_item_id
        ),
    );
    context
}

fn first_text(result: &Value) -> Option<String> {
    result
        .get("content")?
        .as_array()?
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("text"))?
        .get("text")?
        .as_str()
        .map(String::from)
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

fn write_context_snapshot(context: &Value, task_id: &str) -> Result<String, String> {
    let dir = snapshot_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{task_id}.json"));
    let json = serde_json::to_string_pretty(context).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Extract description text from a context value that may be:
/// - a plain string
/// - a rich-text object with `doc_text` field
/// - null / missing
fn extract_description_text(value: &Value) -> &str {
    // Plain string
    if let Some(s) = value.as_str() {
        return s;
    }
    // Rich-text object: prefer doc_text
    if let Some(obj) = value.as_object() {
        if let Some(s) = obj.get("doc_text").and_then(|v| v.as_str()) {
            return s;
        }
    }
    "(no description)"
}

fn build_handoff_message(
    item: &FeishuProjectInboxItem,
    context: &Value,
    task_id: &str,
    snapshot_path: &str,
) -> BridgeMessage {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let desc = extract_description_text(&context["description"]);
    let status = context["status"].as_str().unwrap_or("unknown");
    let priority = context["priority"].as_str().unwrap_or("");
    let classification = context["classification"].as_str().unwrap_or("");
    let mut summary = format!(
        "Feishu Project bug assigned.\n\n\
         **[{}]** {}\n\
         Status: {} | Priority: {} | Classification: {}\n\
         Source: {}\n\n\
         **Description:**\n{}\n",
        item.work_item_id,
        item.title,
        status,
        priority,
        classification,
        item.source_url,
        desc,
    );
    if let Some(comments) = context["comments"].as_array() {
        if !comments.is_empty() {
            summary.push_str("\n**Comments:**\n");
            for c in comments {
                let author = c["author"].as_str().unwrap_or("?");
                let content = c["content"].as_str().unwrap_or("");
                let time = c["created_at"].as_str().unwrap_or("");
                summary.push_str(&format!("- [{time}] {author}: {content}\n"));
            }
        }
    }
    summary.push_str(&format!(
        "\nTask: `{task_id}`\n\
         Full context in attached snapshot. \
         Start by writing a repair plan, then follow \
         plan \u{2192} execute \u{2192} review \u{2192} CM flow."
    ));
    BridgeMessage {
        id: format!("fp_handoff_{task_id}_{now}"),
        from: "system".into(),
        display_source: Some("feishu_project".into()),
        to: "lead".into(),
        content: summary,
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
