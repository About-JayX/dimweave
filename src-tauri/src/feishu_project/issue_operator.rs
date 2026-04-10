//! Extract operator names from `get_workitem_brief` detail responses.
//!
//! Keeps detail-parsing logic separate from the MQL sync adapter so
//! `mcp_sync.rs` and `runtime.rs` stay within the 200-line guideline.

use super::types::FeishuProjectInboxItem;
use serde_json::Value;

/// Extract operator names from a parsed `get_workitem_brief` detail object.
///
/// Real response shape: `work_item_attribute.role_members` is an **array** of
/// `{"key": "<role_id>", "members": [{"name_cn": "...", ...}]}`.
/// We find the element with `key == "operator"` and collect `name_cn` values.
pub fn parse_operator_names(detail: &Value) -> Vec<String> {
    let Some(roles) = detail
        .pointer("/work_item_attribute/role_members")
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    let Some(op_entry) = roles.iter().find(|r| {
        r.get("key").and_then(|k| k.as_str()) == Some("operator")
    }) else {
        return Vec::new();
    };
    let Some(members) = op_entry.get("members").and_then(|m| m.as_array()) else {
        return Vec::new();
    };
    members
        .iter()
        .filter_map(|u| {
            u.get("name")
                .or_else(|| u.get("name_cn"))
                .and_then(|n| n.as_str())
        })
        .filter(|n| !n.is_empty())
        .map(String::from)
        .collect()
}

/// Derive unique sorted team-member names from items' `assignee_label` fields.
pub fn derive_team_members(items: &[FeishuProjectInboxItem]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut names = Vec::new();
    for item in items {
        if let Some(label) = &item.assignee_label {
            for name in label.split(", ") {
                if !name.is_empty() && seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}
