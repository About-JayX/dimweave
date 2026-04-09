use super::api::{should_continue_paging, ApiResponse, PollResult, WorkItemRaw, WorkItemTypeInfo};
use super::store::FeishuProjectStore;
use super::types::{FeishuProjectInboxItem, IngressSource};

// ── Pagination termination logic ─────────────────────────────────────────────

#[test]
fn paging_continues_when_more_pages_remain() {
    // page 1 of 3 (50 fetched, 120 total)
    assert!(should_continue_paging(1, 50, 120, false));
    // page 2 of 3 (100 fetched, 120 total)
    assert!(should_continue_paging(2, 50, 120, false));
}

#[test]
fn paging_stops_when_fetched_reaches_total() {
    // page 3 of 3 (150 fetched >= 120 total)
    assert!(!should_continue_paging(3, 50, 120, false));
    // exact boundary (50 fetched == 50 total)
    assert!(!should_continue_paging(1, 50, 50, false));
}

#[test]
fn paging_stops_on_empty_page() {
    assert!(!should_continue_paging(1, 50, 100, true));
}

#[test]
fn paging_stops_when_total_is_zero() {
    assert!(!should_continue_paging(1, 50, 0, false));
}

// ── Multi-page accumulation via store ────────────────────────────────────────

fn make_item(id: i64, title: &str) -> FeishuProjectInboxItem {
    FeishuProjectInboxItem {
        record_id: format!("proj_{id}"),
        project_key: "proj".into(),
        work_item_id: id.to_string(),
        work_item_type_key: "bug".into(),
        title: title.into(),
        status_label: None,
        assignee_label: None,
        updated_at: 0,
        source_url: String::new(),
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Poll,
        last_event_uuid: None,
    }
}

#[test]
fn multi_page_items_accumulate_in_store() {
    let mut store = FeishuProjectStore::default();
    // Simulate page 1: items 1-3
    let page1 = vec![make_item(1, "A"), make_item(2, "B"), make_item(3, "C")];
    for item in page1 {
        store.upsert(item);
    }
    assert_eq!(store.items.len(), 3);

    // Simulate page 2: items 4-5
    let page2 = vec![make_item(4, "D"), make_item(5, "E")];
    for item in page2 {
        store.upsert(item);
    }
    assert_eq!(store.items.len(), 5);
    assert_eq!(store.items[0].title, "A");
    assert_eq!(store.items[4].title, "E");
}

#[test]
fn re_poll_updates_existing_items_without_duplicating() {
    let mut store = FeishuProjectStore::default();
    // Initial poll
    for i in 1..=3 {
        store.upsert(make_item(i, &format!("v1_{i}")));
    }
    assert_eq!(store.items.len(), 3);

    // Mark item 2 as ignored + linked
    store.items[1].ignored = true;
    store.items[1].linked_task_id = Some("task_x".into());

    // Re-poll with updated titles
    for i in 1..=3 {
        store.upsert(make_item(i, &format!("v2_{i}")));
    }
    assert_eq!(store.items.len(), 3);
    assert_eq!(store.items[0].title, "v2_1");
    assert_eq!(store.items[1].title, "v2_2");
    // Preserved workflow state
    assert!(store.items[1].ignored);
    assert_eq!(store.items[1].linked_task_id.as_deref(), Some("task_x"));
}

// ── Response parsing ─────────────────────────────────────────────────────────

#[test]
fn parse_filter_response_extracts_items_and_total() {
    let json = r#"{
        "err_code": 0,
        "data": [
            { "id": 1, "name": "Bug A", "work_item_type_key": "bug" },
            { "id": 2, "name": "Story B", "work_item_type_key": "story" }
        ],
        "pagination": { "total": 55 }
    }"#;
    let resp: ApiResponse<Vec<WorkItemRaw>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.err_code, 0);
    let data = resp.data.unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(data[0].name, "Bug A");
    assert_eq!(resp.pagination.unwrap().total, 55);
}

#[test]
fn parse_types_response() {
    let json = r#"{
        "err_code": 0,
        "data": [
            { "type_key": "bug" },
            { "type_key": "story" },
            { "type_key": "task" }
        ]
    }"#;
    let resp: ApiResponse<Vec<WorkItemTypeInfo>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.err_code, 0);
    let keys: Vec<String> = resp.data.unwrap().into_iter().map(|t| t.type_key).collect();
    assert_eq!(keys, vec!["bug", "story", "task"]);
}

#[test]
fn parse_error_response() {
    let json = r#"{ "err_code": 10022, "err_msg": "token expired" }"#;
    let resp: ApiResponse<Vec<WorkItemRaw>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.err_code, 10022);
    assert_eq!(resp.err_msg.as_deref(), Some("token expired"));
    assert!(resp.data.is_none());
}

// ── Truncation detection ─────────────────────────────────────────────────────

#[test]
fn poll_result_detects_truncation_above_2000() {
    // Simulated: total=2500 but we only got 2000 items
    let result = PollResult {
        items: Vec::new(),
        truncated: true,
        api_total: 2500,
    };
    assert!(result.truncated);
    assert_eq!(result.api_total, 2500);
}

#[test]
fn poll_result_not_truncated_below_2000() {
    let result = PollResult {
        items: Vec::new(),
        truncated: false,
        api_total: 150,
    };
    assert!(!result.truncated);
}
