//! Extract operator names from `get_workitem_brief` detail responses.
//!
//! Keeps detail-parsing logic separate from the MQL sync adapter so
//! `mcp_sync.rs` and `runtime.rs` stay within the 200-line guideline.

use super::types::FeishuProjectInboxItem;
use serde_json::Value;

/// Extract operator names from a parsed `get_workitem_brief` detail object.
///
/// Navigates `work_item_attribute.role_members.operator` and collects `name_cn`.
/// Returns an empty vec if the path is missing or contains no valid names.
pub fn parse_operator_names(detail: &Value) -> Vec<String> {
    let Some(ops) = detail
        .pointer("/work_item_attribute/role_members/operator")
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };
    ops.iter()
        .filter_map(|u| u.get("name_cn").and_then(|n| n.as_str()))
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
