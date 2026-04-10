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

/// Build MQL with optional server-side `work_item_status` filter.
pub fn build_filtered_mql(workspace: &str, offset: u32, status: Option<&str>) -> String {
    let where_clause = match status {
        Some(s) if !s.is_empty() => format!(" WHERE work_item_status = \"{s}\""),
        _ => String::new(),
    };
    format!(
        "SELECT work_item_id, name, priority, bug_classification \
         FROM {ws}.issue{wh} LIMIT {offset}, {limit}",
        ws = workspace,
        wh = where_clause,
        offset = offset,
        limit = MQL_PAGE_SIZE,
    )
}

/// Fetch one raw page with optional status filter, returning parsed items.
pub async fn fetch_filtered_page(
    client: &McpClient,
    workspace: &str,
    offset: u32,
    status: Option<&str>,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let mql = build_filtered_mql(workspace, offset, status);
    let args = serde_json::json!({"project_key": workspace, "mql": mql});
    let result = client.call_tool("search_by_mql", args).await?;
    issue_query_parse::parse_mql_items(&result, workspace)
}

/// Progressive assignee-filtered scan: fetch raw pages starting at `cursor`,
/// enrich with operator detail, filter by assignee, collect up to one page of
/// matches, and advance the cursor.
pub async fn scan_assignee_page(
    client: &McpClient,
    workspace: &str,
    cursor: &mut IssueQueryCursor,
    target_count: usize,
) -> Result<Vec<FeishuProjectInboxItem>, String> {
    let mut collected = Vec::new();
    while collected.len() < target_count && !cursor.exhausted {
        let status_filter = cursor.filter.status.as_deref();
        let mut page =
            fetch_filtered_page(client, workspace, cursor.raw_offset, status_filter).await?;
        if (page.len() as u32) < MQL_PAGE_SIZE {
            cursor.exhausted = true;
        }
        cursor.raw_offset += page.len() as u32;
        super::mcp_sync::enrich_issues_with_operators(client, workspace, &mut page).await;
        if let Some(ref target) = cursor.filter.assignee {
            for item in page {
                if matches_assignee(&item, target) {
                    collected.push(item);
                    if collected.len() >= target_count {
                        break;
                    }
                }
            }
        } else {
            collected.extend(page);
        }
    }
    Ok(collected)
}

fn matches_assignee(item: &FeishuProjectInboxItem, target: &str) -> bool {
    item.assignee_label
        .as_ref()
        .map(|label| label.split(", ").any(|n| n == target))
        .unwrap_or(false)
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

/// Re-export for lifecycle callers.
pub use super::issue_query_team::fetch_team_member_names;

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::IngressSource;

    #[test]
    fn build_filtered_mql_without_status() {
        let mql = build_filtered_mql("myws", 0, None);
        assert!(mql.contains("FROM myws.issue LIMIT 0, 50"));
        assert!(!mql.contains("WHERE"));
    }

    #[test]
    fn build_filtered_mql_with_status() {
        let mql = build_filtered_mql("myws", 50, Some("已关闭"));
        assert!(mql.contains("WHERE work_item_status = \"已关闭\""));
        assert!(mql.contains("LIMIT 50, 50"));
    }

    #[test]
    fn matches_assignee_exact() {
        let item = FeishuProjectInboxItem {
            assignee_label: Some("Alice, Bob".into()),
            ..test_item()
        };
        assert!(matches_assignee(&item, "Alice"));
        assert!(matches_assignee(&item, "Bob"));
        assert!(!matches_assignee(&item, "Charlie"));
    }

    #[test]
    fn matches_assignee_none() {
        let item = FeishuProjectInboxItem {
            assignee_label: None,
            ..test_item()
        };
        assert!(!matches_assignee(&item, "Alice"));
    }

    #[test]
    fn cursor_defaults() {
        let c = IssueQueryCursor::default();
        assert_eq!(c.raw_offset, 0);
        assert!(!c.exhausted);
        assert_eq!(c.filter, IssueFilter::default());
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
