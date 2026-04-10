use super::*;
use crate::feishu_project::tool_catalog::{McpToolCatalog, McpToolInfo};

fn catalog_with(names: &[&str]) -> McpToolCatalog {
    McpToolCatalog {
        tools: names
            .iter()
            .map(|n| McpToolInfo {
                name: n.to_string(),
                description: None,
                input_schema: None,
            })
            .collect(),
    }
}

#[test]
fn resolve_listing_tool_exact_match() {
    let cat = catalog_with(&["get_spaces", "search_work_item", "add_comment"]);
    assert_eq!(resolve_listing_tool(&cat).unwrap(), "search_work_item");
}

#[test]
fn resolve_listing_tool_no_substring_fallback() {
    // find_tool is exact-match only; substring names do not match
    let cat = catalog_with(&["get_spaces", "search_project_work_items", "add_comment"]);
    assert!(resolve_listing_tool(&cat).is_err());
}

#[test]
fn resolve_listing_tool_empty_catalog_error() {
    let cat = McpToolCatalog::default();
    let err = resolve_listing_tool(&cat).unwrap_err();
    assert!(err.contains("empty"));
}

#[test]
fn resolve_listing_tool_no_match_error() {
    let cat = catalog_with(&["get_spaces", "add_comment"]);
    let err = resolve_listing_tool(&cat).unwrap_err();
    assert!(err.contains("no listing tool found"));
    assert!(err.contains("2 tools"));
}

#[test]
fn parse_tool_result_array_of_objects() {
    let result = serde_json::json!([
        {"id": 1001, "name": "Bug A", "work_item_type_key": "bug"},
        {"id": 1002, "name": "Bug B", "work_item_type_key": "story"}
    ]);
    let items = parse_tool_result(&result, "proj").unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].work_item_id, "1001");
    assert_eq!(items[0].title, "Bug A");
    assert_eq!(items[0].last_ingress, IngressSource::Mcp);
    assert_eq!(items[1].work_item_type_key, "story");
}

#[test]
fn parse_tool_result_mcp_content_wrapped() {
    let result = serde_json::json!({
        "content": [{
            "type": "text",
            "text": "[{\"id\":\"42\",\"name\":\"Crash\",\"work_item_type_key\":\"bug\"}]"
        }]
    });
    let items = parse_tool_result(&result, "proj").unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].work_item_id, "42");
}

#[test]
fn parse_tool_result_data_wrapped() {
    let result = serde_json::json!({"data": [{"id": "99", "name": "X"}]});
    let items = parse_tool_result(&result, "proj").unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].work_item_id, "99");
}

#[test]
fn parse_tool_result_unexpected_shape_error() {
    let result = serde_json::json!({"status": "ok"});
    let err = parse_tool_result(&result, "proj").unwrap_err();
    assert!(err.contains("unexpected"));
}

#[test]
fn parse_tool_result_preserves_mcp_ingress() {
    let result = serde_json::json!([{"id": "1", "name": "T"}]);
    let items = parse_tool_result(&result, "p").unwrap();
    assert_eq!(items[0].last_ingress, IngressSource::Mcp);
    assert!(!items[0].ignored);
    assert!(items[0].linked_task_id.is_none());
}

#[test]
fn try_parse_item_skips_missing_id() {
    let raw = serde_json::json!({"name": "No ID item"});
    assert!(try_parse_item(&raw, "proj").is_none());
}

// ── Operator-first assignee + query-level fallback tests ──────

fn mql_user_field(key: &str, name: &str) -> serde_json::Value {
    serde_json::json!({"key": key, "value": {"user_value_list": [{"name_cn": name}]}})
}

#[test]
fn parse_mql_item_prefers_operator_over_current_status_operator() {
    let raw = serde_json::json!({"moql_field_list": [
        {"key": "work_item_id", "value": {"long_value": 100}},
        {"key": "name", "value": {"string_value": "Bug X"}},
        mql_user_field("operator", "Alice"),
        mql_user_field("current_status_operator", "Bob"),
    ]});
    let item = parse_mql_item(&raw, "proj").unwrap();
    assert_eq!(item.assignee_label.as_deref(), Some("Alice"));
}

#[test]
fn parse_mql_item_falls_back_to_current_status_operator() {
    let raw = serde_json::json!({"moql_field_list": [
        {"key": "work_item_id", "value": {"long_value": 200}},
        {"key": "name", "value": {"string_value": "Bug Y"}},
        mql_user_field("current_status_operator", "Bob"),
    ]});
    let item = parse_mql_item(&raw, "proj").unwrap();
    assert_eq!(item.assignee_label.as_deref(), Some("Bob"));
}

#[test]
fn sync_issues_mql_selects_operator() {
    let mql = build_issues_mql("PROJ", 0);
    assert!(mql.contains("operator"), "{mql}");
    assert!(!mql.contains("current_status_operator"), "{mql}");
}

#[test]
fn team_member_mql_groups_by_operator() {
    let mql = build_team_members_mql("PROJ");
    assert!(mql.contains("operator"), "{mql}");
    assert!(!mql.contains("current_status_operator"), "{mql}");
}

#[test]
fn legacy_issues_mql_uses_current_status_operator() {
    let mql = build_issues_mql_legacy("PROJ", 0);
    assert!(mql.contains("current_status_operator"), "{mql}");
}

#[test]
fn legacy_team_member_mql_uses_current_status_operator() {
    let mql = build_team_members_mql_legacy("PROJ");
    assert!(mql.contains("GROUP BY current_status_operator"), "{mql}");
}
