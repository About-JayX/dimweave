use super::*;

#[test]
fn raw_to_inbox_item_maps_fields_correctly() {
    let raw = WorkItemRaw {
        id: 12345,
        name: "Crash on launch".into(),
        work_item_type_key: "bug".into(),
        sub_stage: Some("Open".into()),
        created_by: Some("alice".into()),
        updated_by: Some("bob".into()),
        updated_at: Some(1700000000000),
        simple_name: Some("myspace".into()),
    };
    let item = raw_to_inbox_item(raw, "proj_key");
    assert_eq!(item.record_id, "proj_key_12345");
    assert_eq!(item.work_item_id, "12345");
    assert_eq!(item.title, "Crash on launch");
    assert_eq!(item.work_item_type_key, "bug");
    assert_eq!(item.status_label.as_deref(), Some("Open"));
    assert_eq!(item.assignee_label.as_deref(), Some("bob"));
    assert_eq!(item.updated_at, 1700000000000);
    assert!(item.source_url.contains("myspace/issues/12345"));
    assert_eq!(item.last_ingress, IngressSource::Poll);
    assert!(!item.ignored);
    assert!(item.linked_task_id.is_none());
}

#[test]
fn raw_to_inbox_item_falls_back_to_created_by() {
    let raw = WorkItemRaw {
        id: 1,
        name: "t".into(),
        work_item_type_key: "story".into(),
        sub_stage: None,
        created_by: Some("alice".into()),
        updated_by: None,
        updated_at: None,
        simple_name: None,
    };
    let item = raw_to_inbox_item(raw, "p");
    assert_eq!(item.assignee_label.as_deref(), Some("alice"));
    assert!(item.source_url.contains("p/issues/1"));
}
