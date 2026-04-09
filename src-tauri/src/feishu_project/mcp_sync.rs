//! MCP-to-inbox sync adapter.
//!
//! Resolves work-item listing tools from the discovered catalog, calls them,
//! and maps results into `FeishuProjectInboxItem`. If the real tool catalog
//! is unavailable (e.g. no token), errors are surfaced explicitly.

use super::mcp_client::McpClient;
use super::tool_catalog::McpToolCatalog;
use super::types::{FeishuProjectInboxItem, IngressSource};
use serde_json::Value;

/// Known tool name patterns for work-item listing.
/// These will be matched against the real catalog once available.
const LISTING_TOOL_PATTERNS: &[&str] = &[
    "search_work_item",
    "list_work_item",
    "filter_work_item",
    "get_work_item",
    "query_work_item",
];

/// Resolve the best work-item listing tool from the catalog.
pub fn resolve_listing_tool(catalog: &McpToolCatalog) -> Result<String, String> {
    if catalog.tools.is_empty() {
        return Err("MCP tool catalog is empty — cannot sync work items".into());
    }
    // Exact match first
    for pattern in LISTING_TOOL_PATTERNS {
        if let Some(tool) = catalog.find_tool(pattern) {
            return Ok(tool.name.clone());
        }
    }
    // Substring match: find any tool whose name contains a listing keyword
    for tool in &catalog.tools {
        let lower = tool.name.to_lowercase();
        if (lower.contains("work_item") || lower.contains("workitem"))
            && (lower.contains("search")
                || lower.contains("list")
                || lower.contains("filter")
                || lower.contains("query"))
        {
            return Ok(tool.name.clone());
        }
    }
    Err(format!(
        "no work-item listing tool found in catalog ({} tools: [{}])",
        catalog.tool_count(),
        catalog
            .tool_names()
            .into_iter()
            .take(10)
            .collect::<Vec<_>>()
            .join(", "),
    ))
}

/// Run a full MCP sync cycle: call the listing tool and map results.
pub async fn run_mcp_sync(
    client: &McpClient,
    workspace_hint: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let tool_name = resolve_listing_tool(&client.catalog)?;
    let args = build_listing_args(workspace_hint);
    let result = client.call_tool(&tool_name, args).await?;
    parse_tool_result(&result, workspace_hint)
}

fn build_listing_args(workspace_hint: &str) -> Value {
    let mut args = serde_json::Map::new();
    if !workspace_hint.is_empty() {
        args.insert("project_key".into(), Value::String(workspace_hint.into()));
        args.insert("space_id".into(), Value::String(workspace_hint.into()));
    }
    Value::Object(args)
}

/// Parse the tool result into inbox items.
/// Tries multiple result shapes to handle unknown schemas gracefully.
pub fn parse_tool_result(
    result: &Value,
    fallback_project: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    // MCP tool results wrap content in a `content` array with `text` entries
    let data = extract_content_text(result)
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
                    return Some(vec![parsed]);
                }
            }
        }
    }
    None
}

/// Try to parse a single JSON object into an inbox item.
fn try_parse_item(raw: &Value, fallback_project: &str) -> Option<FeishuProjectInboxItem> {
    let id = json_str(raw, &["id", "work_item_id", "ID"])?;
    let title = json_str(raw, &["name", "title", "summary"]).unwrap_or_default();
    let type_key = json_str(raw, &["work_item_type_key", "type_key", "type"])
        .unwrap_or_else(|| "unknown".into());
    let project = json_str(raw, &["project_key", "space_id", "project"])
        .unwrap_or_else(|| fallback_project.into());
    Some(FeishuProjectInboxItem {
        record_id: format!("{project}_{id}"),
        project_key: project.clone(),
        work_item_id: id.clone(),
        work_item_type_key: type_key,
        title,
        status_label: json_str(raw, &["sub_stage", "status", "state"]),
        assignee_label: json_str(raw, &["updated_by", "assignee", "owner"]),
        updated_at: json_u64(raw, &["updated_at", "update_time"]),
        source_url: json_str(raw, &["url", "link"])
            .unwrap_or_else(|| format!("https://project.feishu.cn/{project}/issues/{id}")),
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
