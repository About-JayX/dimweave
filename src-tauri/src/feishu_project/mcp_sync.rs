//! MCP-to-inbox sync adapter.
//!
//! Resolves work-item listing tools from the discovered catalog, calls them,
//! and maps results into `FeishuProjectInboxItem`. If the real tool catalog
//! is unavailable (e.g. no token), errors are surfaced explicitly.

use super::mcp_client::McpClient;
use super::tool_catalog::McpToolCatalog;
use super::types::{FeishuProjectInboxItem, FeishuSyncMode, IngressSource};
use serde_json::Value;

/// Run a full MCP sync cycle based on the configured sync mode.
pub async fn run_mcp_sync(
    client: &McpClient,
    workspace_hint: &str,
    sync_mode: FeishuSyncMode,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    match sync_mode {
        FeishuSyncMode::Todo => sync_todo(client, workspace_hint).await,
        FeishuSyncMode::Issues => sync_issues(client, workspace_hint).await,
    }
}

async fn sync_todo(
    client: &McpClient,
    workspace_hint: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let args = serde_json::json!({"action": "todo", "page_num": 1});
    let result = client.call_tool("list_todo", args).await?;
    parse_tool_result(&result, workspace_hint)
}

const MQL_PAGE_SIZE: u32 = 50;

/// Build issue-list MQL. Assignee data is sourced from `get_workitem_brief`
/// detail (role_members.operator), not from MQL fields.
pub fn build_issues_mql(workspace: &str, offset: u32) -> String {
    format!(
        "SELECT work_item_id, name, priority, bug_classification \
         FROM {ws}.issue LIMIT {offset}, {limit}",
        ws = workspace,
        offset = offset,
        limit = MQL_PAGE_SIZE,
    )
}

async fn sync_issues(
    client: &McpClient,
    workspace_hint: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    sync_issues_page(client, workspace_hint, 0).await
}

/// Fetch a single page of issues at the given offset.
pub async fn sync_issues_page(
    client: &McpClient,
    workspace_hint: &str,
    offset: u32,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    if workspace_hint.is_empty() {
        return Err("workspace_hint required for issue sync".into());
    }
    let mql = build_issues_mql(workspace_hint, offset);
    let args = serde_json::json!({"project_key": workspace_hint, "mql": mql});
    let result = client.call_tool("search_by_mql", args).await?;
    parse_mql_items(&result, workspace_hint)
}

const ENRICH_CONCURRENCY: usize = 6;

/// Build arguments for `get_workitem_brief` detail call.
/// `work_item_id` must be a string (not integer) per live MCP evidence.
fn build_detail_args(project_key: &str, work_item_id: &str) -> Value {
    serde_json::json!({
        "project_key": project_key,
        "work_item_id": work_item_id,
        "fields": ["description", "priority", "bug_classification", "issue_stage"],
    })
}

/// Fetch operator names for a single work item via `get_workitem_brief`.
async fn fetch_operator_names(
    client: &McpClient,
    project_key: &str,
    work_item_id: &str,
) -> Vec<String> {
    let args = build_detail_args(project_key, work_item_id);
    let Ok(result) = client.call_tool("get_workitem_brief", args).await else {
        return Vec::new();
    };
    let Some(text) = extract_first_text(&result) else {
        return Vec::new();
    };
    let Ok(detail) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    super::issue_operator::parse_operator_names(&detail)
}

/// Enrich issue items with operator names from `get_workitem_brief` detail.
/// Uses bounded concurrency to avoid sequential latency on large lists.
pub async fn enrich_issues_with_operators(
    client: &McpClient,
    project_key: &str,
    items: &mut Vec<FeishuProjectInboxItem>,
) {
    use futures_util::stream::{self, StreamExt};

    let targets: Vec<(usize, String)> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.work_item_id.starts_with("mql_"))
        .map(|(i, item)| (i, item.work_item_id.clone()))
        .collect();

    let results: Vec<(usize, Vec<String>)> = stream::iter(targets)
        .map(|(idx, wid)| async move {
            let names = fetch_operator_names(client, project_key, &wid).await;
            (idx, names)
        })
        .buffer_unordered(ENRICH_CONCURRENCY)
        .collect()
        .await;

    for (idx, names) in results {
        if !names.is_empty() {
            items[idx].assignee_label = Some(names.join(", "));
        }
    }
}

/// Parse MQL response data into inbox items.
fn parse_mql_items(
    result: &Value,
    fallback_project: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let text = extract_first_text(result)
        .ok_or_else(|| "MQL response missing text content".to_string())?;
    let parsed: Value = serde_json::from_str(&text)
        .map_err(|e| format!("MQL response not valid JSON: {e}"))?;

    let data_obj = parsed.get("data").and_then(|d| d.as_object());
    let Some(data) = data_obj else {
        return Err(format!(
            "MQL response missing 'data' object: {}",
            truncate_json(&parsed, 200),
        ));
    };
    let mut items = Vec::new();
    for (_gid, group_items) in data {
        let Some(arr) = group_items.as_array() else {
            continue;
        };
        for raw_item in arr {
            if let Some(item) = parse_mql_item(raw_item, fallback_project) {
                items.push(item);
            }
        }
    }
    Ok(items)
}

/// Parse a single MQL item with `moql_field_list` fields.
fn parse_mql_item(raw: &Value, fallback_project: &str) -> Option<FeishuProjectInboxItem> {
    let fields = raw.get("moql_field_list")?.as_array()?;
    let mut name = String::new();
    let mut priority = String::new();
    let mut classification = String::new();
    let mut work_item_id = String::new();
    for f in fields {
        let key = f.get("key").and_then(|k| k.as_str()).unwrap_or("");
        match key {
            "name" => {
                name = f["value"]["string_value"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
            }
            "priority" => {
                priority = f["value"]["key_label_value"]["label"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
            }
            "bug_classification" => {
                classification = f["value"]["key_label_value"]["label"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
            }
            "work_item_id" | "id" => {
                work_item_id = f["value"]["long_value"]
                    .as_i64()
                    .map(|n| n.to_string())
                    .or_else(|| f["value"]["string_value"].as_str().map(String::from))
                    .unwrap_or_default();
            }
            _ => {}
        }
    }
    // MQL items may lack explicit work_item_id field; use item index as fallback
    if work_item_id.is_empty() {
        // Generate a stable id from name hash
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        work_item_id = format!("mql_{}", hasher.finish());
    }
    let status = if !priority.is_empty() && !classification.is_empty() {
        Some(format!("{priority} · {classification}"))
    } else if !priority.is_empty() {
        Some(priority)
    } else if !classification.is_empty() {
        Some(classification)
    } else {
        None
    };
    let source_url = format!(
        "https://project.feishu.cn/{}/issue/detail/{}",
        fallback_project, work_item_id,
    );
    Some(FeishuProjectInboxItem {
        record_id: format!("{fallback_project}_{work_item_id}"),
        project_key: fallback_project.to_string(),
        work_item_id,
        work_item_type_key: "issue".to_string(),
        title: name,
        status_label: status,
        assignee_label: None, // filled by detail enrichment, not MQL
        updated_at: 0,
        source_url,
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Mcp,
        last_event_uuid: None,
    })
}

fn mql_field_string_value(f: &Value) -> String {
    f["value"]["string_value"]
        .as_str()
        .or_else(|| f["value"]["key_label_value"]["key"].as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract the first text content entry from an MCP response.
fn extract_first_text(result: &Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    for entry in content {
        if entry.get("type").and_then(|t| t.as_str()) == Some("text") {
            return entry.get("text").and_then(|t| t.as_str()).map(String::from);
        }
    }
    None
}

// ── Legacy resolve (kept for resolve_listing_tool compat in tests) ───

/// Known tool name patterns for work-item listing, ordered by preference.
const LISTING_TOOL_PATTERNS: &[&str] = &[
    "list_todo",
    "search_by_mql",
    "search_work_item",
    "list_work_item",
];

/// Resolve the best work-item listing tool from the catalog.
pub fn resolve_listing_tool(catalog: &McpToolCatalog) -> Result<String, String> {
    if catalog.tools.is_empty() {
        return Err("MCP tool catalog is empty — cannot sync work items".into());
    }
    for pattern in LISTING_TOOL_PATTERNS {
        if let Some(tool) = catalog.find_tool(pattern) {
            return Ok(tool.name.clone());
        }
    }
    Err(format!(
        "no listing tool found in catalog ({} tools)",
        catalog.tool_count(),
    ))
}

/// Parse the tool result into inbox items.
/// Tries multiple result shapes to handle unknown schemas gracefully.
pub fn parse_tool_result(
    result: &Value,
    fallback_project: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    // MCP tool results wrap content in a `content` array with `text` entries
    let data = extract_content_text(result)
        .or_else(|| result.get("list").and_then(|v| v.as_array()).cloned())
        .or_else(|| result.as_array().cloned())
        .or_else(|| result.get("items").and_then(|v| v.as_array()).cloned())
        .or_else(|| result.get("data").and_then(|v| v.as_array()).cloned())
        .or_else(|| result.get("work_items").and_then(|v| v.as_array()).cloned());
    let items_arr = match data {
        Some(arr) => arr,
        None => {
            return Err(format!(
                "unexpected MCP tool result shape — expected array or content: {}",
                truncate_json(result, 200),
            ));
        }
    };
    let mut items = Vec::new();
    for raw in &items_arr {
        if let Some(item) = try_parse_item(raw, fallback_project) {
            items.push(item);
        }
    }
    Ok(items)
}

/// Extract text content from MCP content-wrapped results.
fn extract_content_text(result: &Value) -> Option<Vec<Value>> {
    let content = result.get("content")?.as_array()?;
    for entry in content {
        if entry.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = entry.get("text").and_then(|t| t.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                    if let Some(arr) = parsed.as_array() {
                        return Some(arr.clone());
                    }
                    // Handle `{"list": [...]}` wrapper (e.g. list_todo)
                    if let Some(arr) = parsed.get("list").and_then(|v| v.as_array()) {
                        return Some(arr.clone());
                    }
                    return Some(vec![parsed]);
                }
            }
        }
    }
    None
}

/// Try to parse a single JSON object into an inbox item.
/// Handles both flat items and nested `{work_item_info: {...}, project_key}` shapes.
fn try_parse_item(raw: &Value, fallback_project: &str) -> Option<FeishuProjectInboxItem> {
    // Flatten nested work_item_info if present
    let inner = raw.get("work_item_info").unwrap_or(raw);
    let id = json_str(inner, &["work_item_id", "id", "ID"])?;
    let title = json_str(inner, &["work_item_name", "name", "title", "summary"])
        .unwrap_or_default();
    let type_key = json_str(inner, &["work_item_type_key", "type_key", "type"])
        .unwrap_or_else(|| "unknown".into());
    let project = json_str(raw, &["project_key", "space_id", "project"])
        .or_else(|| json_str(inner, &["project_key"]))
        .unwrap_or_else(|| fallback_project.into());
    let source_url = json_str(raw, &["url", "link"]).unwrap_or_else(|| {
        let slug = if fallback_project.is_empty() { project.as_str() } else { fallback_project };
        format!("https://project.feishu.cn/{slug}/{}/detail/{id}", type_key)
    });
    Some(FeishuProjectInboxItem {
        record_id: format!("{project}_{id}"),
        project_key: project.clone(),
        work_item_id: id.clone(),
        work_item_type_key: type_key,
        title,
        status_label: json_str(raw, &["sub_stage", "status", "state"])
            .or_else(|| {
                raw.get("node_info")
                    .and_then(|n| json_str(n, &["node_name", "node_state_key"]))
            }),
        assignee_label: json_str(raw, &["updated_by", "assignee", "owner"]),
        updated_at: json_u64(raw, &["updated_at", "update_time"])
            .max(json_u64(inner, &["updated_at", "update_time"])),
        source_url,
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Mcp,
        last_event_uuid: None,
    })
}

/// Get a string from any of several possible field names.
fn json_str(val: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = val.get(key) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
            if let Some(n) = v.as_i64() {
                return Some(n.to_string());
            }
        }
    }
    None
}

fn json_u64(val: &Value, keys: &[&str]) -> u64 {
    for key in keys {
        if let Some(n) = val.get(key).and_then(|v| v.as_u64()) {
            return n;
        }
    }
    0
}

fn truncate_json(val: &Value, max: usize) -> String {
    let s = val.to_string();
    if s.len() > max { format!("{}...", &s[..max]) } else { s }
}

#[cfg(test)]
#[path = "mcp_sync_tests.rs"]
mod tests;
