//! Filtered issue query: server-side status MQL, progressive assignee scan,
//! team-membership-based assignee options, and cursor-based pagination.

use super::issue_query_parse;
use super::mcp_client::McpClient;
use super::types::{FeishuProjectInboxItem, IssueFilter};

const MQL_PAGE_SIZE: u32 = 50;

/// Cursor state for filtered issue pagination.
#[derive(Debug, Clone, Default)]
pub struct IssueQueryCursor {
    /// Last filter tuple — reset cursor when filter changes.
    pub filter: IssueFilter,
    /// Raw MQL offset (not the filtered result count).
    pub raw_offset: u32,
    /// True when the last raw page returned fewer than MQL_PAGE_SIZE items.
    pub exhausted: bool,
}

/// Build MQL with optional server-side `work_item_status` + `current_status_operator` filters.
pub fn build_filtered_mql(
    workspace: &str,
    offset: u32,
    status: Option<&str>,
    assignee: Option<&str>,
) -> String {
    let mut conditions = Vec::new();
    if let Some(s) = status {
        if !s.is_empty() {
            conditions.push(format!("work_item_status = \"{s}\""));
        }
    }
    if let Some(a) = assignee {
        if !a.is_empty() {
            conditions.push(format!("current_status_operator IN (\"{a}\")"));
        }
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    format!(
        "SELECT work_item_id, name, priority, bug_classification, current_status_operator \
         FROM {ws}.issue{wh} LIMIT {offset}, {limit}",
        ws = workspace,
        wh = where_clause,
        offset = offset,
        limit = MQL_PAGE_SIZE,
    )
}

/// Fetch one page with optional status + assignee filters, returning parsed items.
pub async fn fetch_filtered_page(
    client: &McpClient,
    workspace: &str,
    offset: u32,
    filter: &IssueFilter,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let mql = build_filtered_mql(
        workspace,
        offset,
        filter.status.as_deref(),
        filter.assignee.as_deref(),
    );
    let args = serde_json::json!({"project_key": workspace, "mql": mql});
    let result = client.call_tool("search_by_mql", args).await?;
    issue_query_parse::parse_mql_items(&result, workspace)
}

/// Fetch distinct status labels via MQL GROUP BY.
pub async fn fetch_status_options(
    client: &McpClient,
    workspace: &str,
) -> Result<Vec<String>, String> {
    let mql = format!(
        "SELECT work_item_status FROM {ws}.issue GROUP BY work_item_status",
        ws = workspace,
    );
    let args = serde_json::json!({"project_key": workspace, "mql": mql});
    let result = client.call_tool("search_by_mql", args).await?;
    issue_query_parse::parse_status_group_by(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::IngressSource;

    #[test]
    fn build_filtered_mql_without_filters() {
        let mql = build_filtered_mql("myws", 0, None, None);
        assert!(mql.contains("FROM myws.issue LIMIT 0, 50"));
        assert!(!mql.contains("WHERE"));
        assert!(mql.contains("current_status_operator"));
    }

    #[test]
    fn build_filtered_mql_with_status() {
        let mql = build_filtered_mql("myws", 50, Some("已关闭"), None);
        assert!(mql.contains("WHERE work_item_status = \"已关闭\""));
        assert!(mql.contains("LIMIT 50, 50"));
    }

    #[test]
    fn build_filtered_mql_with_assignee() {
        let mql = build_filtered_mql("myws", 0, None, Some("牛丸"));
        assert!(mql.contains("current_status_operator IN (\"牛丸\")"));
        assert!(mql.contains("WHERE"));
    }

    #[test]
    fn build_filtered_mql_with_status_and_assignee() {
        let mql = build_filtered_mql("myws", 0, Some("已关闭"), Some("牛丸"));
        assert!(mql.contains("work_item_status = \"已关闭\""));
        assert!(mql.contains("current_status_operator IN (\"牛丸\")"));
        assert!(mql.contains("AND"));
    }

    #[test]
    fn cursor_defaults() {
        let c = IssueQueryCursor::default();
        assert_eq!(c.raw_offset, 0);
        assert!(!c.exhausted);
        assert_eq!(c.filter, IssueFilter::default());
    }

    #[test]
    fn parse_status_group_by_real_payload() {
        use super::super::issue_query_parse::parse_status_group_by;
        // Real Feishu GROUP BY response: list[].group_infos[].group_name
        let payload = serde_json::json!({
            "content": [{
                "type": "text",
                "text": r#"{"list":[{"group_infos":[{"group_name":"新"}]},{"group_infos":[{"group_name":"重新打开"}]},{"group_infos":[{"group_name":"已关闭"}]},{"group_infos":[{"group_name":"设计如此"}]}]}"#
            }]
        });
        let result = parse_status_group_by(&payload).unwrap();
        assert_eq!(result, vec!["新", "重新打开", "已关闭", "设计如此"]);
    }

    fn test_item() -> FeishuProjectInboxItem {
        FeishuProjectInboxItem {
            record_id: "test_1".into(),
            project_key: "test".into(),
            work_item_id: "1".into(),
            work_item_type_key: "issue".into(),
            title: "Test".into(),
            status_label: None,
            assignee_label: None,
            updated_at: 0,
            source_url: String::new(),
            raw_snapshot_ref: String::new(),
            ignored: false,
            linked_task_id: None,
            last_ingress: IngressSource::Mcp,
            last_event_uuid: None,
        }
    }
}
