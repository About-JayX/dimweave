use super::*;

#[test]
fn dedup_rejects_repeat() {
    let u = format!("dedup_test_{}", uuid::Uuid::new_v4());
    assert!(!is_duplicate(&u));
    assert!(is_duplicate(&u));
}

#[test]
fn dedup_allows_empty_uuid() {
    assert!(!is_duplicate(""));
    assert!(!is_duplicate(""));
}

#[test]
fn build_item_requires_project_key_and_id() {
    let ev = WebhookEvent {
        project_key: None,
        work_item_id: Some(1),
        work_item_type_key: None,
        name: None,
        sub_stage: None,
        updated_by: None,
        updated_at: None,
    };
    assert!(build_item(ev, "u1").is_none());

    let ev2 = WebhookEvent {
        project_key: Some("proj".into()),
        work_item_id: None,
        work_item_type_key: None,
        name: None,
        sub_stage: None,
        updated_by: None,
        updated_at: None,
    };
    assert!(build_item(ev2, "u2").is_none());
}

#[test]
fn build_item_maps_fields() {
    let ev = WebhookEvent {
        project_key: Some("pk".into()),
        work_item_id: Some(42),
        work_item_type_key: Some("bug".into()),
        name: Some("Crash".into()),
        sub_stage: Some("Open".into()),
        updated_by: Some("bob".into()),
        updated_at: Some(1700000000000),
    };
    let item = build_item(ev, "uuid_1").unwrap();
    assert_eq!(item.record_id, "pk_42");
    assert_eq!(item.work_item_id, "42");
    assert_eq!(item.title, "Crash");
    assert_eq!(
        item.last_ingress,
        crate::feishu_project::types::IngressSource::Webhook
    );
    assert_eq!(item.last_event_uuid.as_deref(), Some("uuid_1"));
    assert!(item.source_url.contains("pk/issues/42"));
}

#[test]
fn webhook_body_deserializes_challenge() {
    let json = r#"{ "type": "url_verification", "challenge": "ch_abc", "token": "tok" }"#;
    let body: WebhookBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.msg_type.as_deref(), Some("url_verification"));
    assert_eq!(body.challenge.as_deref(), Some("ch_abc"));
    assert_eq!(body.token.as_deref(), Some("tok"));
}

#[test]
fn webhook_body_deserializes_event() {
    let json = r#"{
        "header": { "token": "t", "uuid": "u" },
        "event": { "project_key": "p", "work_item_id": 1, "name": "Bug" }
    }"#;
    let body: WebhookBody = serde_json::from_str(json).unwrap();
    let h = body.header.unwrap();
    assert_eq!(h.token, "t");
    assert_eq!(h.uuid, "u");
    let ev = body.event.unwrap();
    assert_eq!(ev.project_key.as_deref(), Some("p"));
    assert_eq!(ev.work_item_id, Some(1));
}
