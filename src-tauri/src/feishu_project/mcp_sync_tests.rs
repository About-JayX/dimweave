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

// ── current_status_operator assignee field tests ──────

fn mql_user_field(key: &str, name: &str) -> serde_json::Value {
    serde_json::json!({"key": key, "value": {"user_value_list": [{"name_cn": name}]}})
}

#[test]
fn parse_mql_item_leaves_assignee_empty() {
    // MQL no longer sets assignee; enrichment via get_workitem_brief does.
    let raw = serde_json::json!({"moql_field_list": [
        {"key": "work_item_id", "value": {"long_value": 200}},
        {"key": "name", "value": {"string_value": "Bug Y"}},
        mql_user_field("current_status_operator", "Bob"),
    ]});
    let item = parse_mql_item(&raw, "proj").unwrap();
    assert_eq!(item.assignee_label, None);
}

#[test]
fn parse_mql_item_ignores_unknown_operator_field() {
    // `operator` is not a valid issue field_key; it should be ignored
    let raw = serde_json::json!({"moql_field_list": [
        {"key": "work_item_id", "value": {"long_value": 100}},
        {"key": "name", "value": {"string_value": "Bug X"}},
        mql_user_field("operator", "Alice"),
    ]});
    let item = parse_mql_item(&raw, "proj").unwrap();
    assert_eq!(item.assignee_label, None);
}

#[test]
fn issues_mql_excludes_bare_operator() {
    let mql = build_issues_mql("PROJ", 0);
    assert!(!mql.contains(" operator"), "MQL must not SELECT bare `operator`: {mql}");
}

// ── issue_operator tests ──────────────────────────────────

#[test]
fn parse_operator_names_from_detail() {
    use crate::feishu_project::issue_operator;
    let detail = serde_json::json!({
        "work_item_attribute": {
            "role_members": {
                "operator": [
                    {"name_cn": "Alice", "user_key": "u1"},
                    {"name_cn": "Bob", "user_key": "u2"}
                ],
                "reporter": [
                    {"name_cn": "Charlie", "user_key": "u3"}
                ]
            }
        }
    });
    let names = issue_operator::parse_operator_names(&detail);
    assert_eq!(names, vec!["Alice", "Bob"]);
}

#[test]
fn parse_operator_names_empty_when_missing() {
    use crate::feishu_project::issue_operator;
    let detail = serde_json::json!({"work_item_attribute": {}});
    assert!(issue_operator::parse_operator_names(&detail).is_empty());
}

#[test]
fn parse_operator_names_skips_empty_names() {
    use crate::feishu_project::issue_operator;
    let detail = serde_json::json!({
        "work_item_attribute": {
            "role_members": {
                "operator": [
                    {"name_cn": "Alice"},
                    {"name_cn": ""},
                    {"user_key": "no_name"}
                ]
            }
        }
    });
    assert_eq!(issue_operator::parse_operator_names(&detail), vec!["Alice"]);
}

#[test]
fn derive_team_members_from_items() {
    use crate::feishu_project::issue_operator;
    use crate::feishu_project::types::{FeishuProjectInboxItem, IngressSource};
    let make = |assignee: Option<&str>| FeishuProjectInboxItem {
        record_id: String::new(),
        project_key: String::new(),
        work_item_id: String::new(),
        work_item_type_key: "issue".into(),
        title: String::new(),
        status_label: None,
        assignee_label: assignee.map(String::from),
        updated_at: 0,
        source_url: String::new(),
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Mcp,
        last_event_uuid: None,
    };
    let items = vec![
        make(Some("Alice")),
        make(Some("Bob, Alice")),
        make(None),
    ];
    assert_eq!(issue_operator::derive_team_members(&items), vec!["Alice", "Bob"]);
}
