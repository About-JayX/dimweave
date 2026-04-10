//! MQL response parsing for filtered issue queries.
//! Duplicated from `mcp_sync` private functions because those are not public.

use super::types::{FeishuProjectInboxItem, IngressSource};
use serde_json::Value;

/// Extract the first text content entry from an MCP response.
pub(crate) fn extract_first_text(result: &Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    for entry in content {
        if entry.get("type").and_then(|t| t.as_str()) == Some("text") {
            return entry.get("text").and_then(|t| t.as_str()).map(String::from);
        }
    }
    None
}

/// Parse MQL response into inbox items.
pub(crate) fn parse_mql_items(
    result: &Value,
    fallback_project: &str,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let text = extract_first_text(result)
        .ok_or_else(|| "MQL response missing text content".to_string())?;
    let parsed: Value =
        serde_json::from_str(&text).map_err(|e| format!("MQL response not valid JSON: {e}"))?;
    let data = parsed.get("data").and_then(|d| d.as_object());
    let Some(data) = data else {
        return Err("MQL response missing 'data' object".to_string());
    };
    let mut items = Vec::new();
    for (_gid, group_items) in data {
        if let Some(arr) = group_items.as_array() {
            for raw in arr {
                if let Some(item) = parse_mql_item(raw, fallback_project) {
                    items.push(item);
                }
            }
        }
    }
    Ok(items)
}

fn parse_mql_item(raw: &Value, project: &str) -> Option<FeishuProjectInboxItem> {
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
    if work_item_id.is_empty() {
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
        "https://project.feishu.cn/{project}/issue/detail/{work_item_id}",
    );
    Some(FeishuProjectInboxItem {
        record_id: format!("{project}_{work_item_id}"),
        project_key: project.to_string(),
        work_item_id,
        work_item_type_key: "issue".to_string(),
        title: name,
        status_label: status,
        assignee_label: None,
        updated_at: 0,
        source_url,
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Mcp,
        last_event_uuid: None,
    })
}

/// Parse status GROUP BY response into distinct status labels.
pub(crate) fn parse_status_group_by(
    result: &Value,
) -> Result<Vec<String>, String> {
    let text = extract_first_text(result)
        .ok_or_else(|| "status GROUP BY: missing text content".to_string())?;
    let parsed: Value =
        serde_json::from_str(&text).map_err(|e| format!("status GROUP BY: bad JSON: {e}"))?;
    let data = parsed.get("data").and_then(|d| d.as_object());
    let Some(data) = data else {
        return Ok(Vec::new());
    };
    let mut statuses = Vec::new();
    for (_gid, items) in data {
        if let Some(arr) = items.as_array() {
            for item in arr {
                if let Some(fields) = item.get("moql_field_list").and_then(|f| f.as_array()) {
                    for f in fields {
                        let key = f.get("key").and_then(|k| k.as_str()).unwrap_or("");
                        if key == "work_item_status" {
                            if let Some(label) = f
                                .pointer("/value/key_label_value/label")
                                .and_then(|v| v.as_str())
                            {
                                if !label.is_empty() && !statuses.contains(&label.to_string()) {
                                    statuses.push(label.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(statuses)
}
