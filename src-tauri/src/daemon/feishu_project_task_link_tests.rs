use super::*;
use crate::daemon::state::DaemonState;
use crate::feishu_project::types::{FeishuProjectInboxItem, IngressSource};
use serde_json::{json, Value};

fn sample_item() -> FeishuProjectInboxItem {
    FeishuProjectInboxItem {
        record_id: "rec_1".into(),
        project_key: "proj".into(),
        work_item_id: "1001".into(),
        work_item_type_key: "bug".into(),
        title: "Crash on launch".into(),
        status_label: Some("Open".into()),
        assignee_label: Some("alice".into()),
        updated_at: 10,
        source_url: "https://project.feishu.cn/proj/issues/1001".into(),
        raw_snapshot_ref: "".into(),
        ignored: false,
        linked_task_id: None,
        last_ingress: IngressSource::Poll,
        last_event_uuid: None,
    }
}

fn sample_context() -> Value {
    json!({
        "work_item_id": "1001",
        "name": "Crash on launch",
        "type": "bug",
        "priority": "Open",
        "assignee": "alice",
        "source_url": "https://project.feishu.cn/proj/issues/1001",
        "status": "Open",
        "description": "App crashes on cold start",
    })
}

#[test]
fn write_context_snapshot_creates_json_file() {
    let context = sample_context();
    let task_id = format!("test_snap_{}", chrono::Utc::now().timestamp_millis());
    let path = write_context_snapshot(&context, &task_id).unwrap();
    assert!(std::path::Path::new(&path).exists());
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["work_item_id"], "1001");
    assert_eq!(parsed["name"], "Crash on launch");
    assert_eq!(parsed["description"], "App crashes on cold start");
    let _ = std::fs::remove_file(path);
}

#[test]
fn build_handoff_message_has_correct_fields() {
    let item = sample_item();
    let context = sample_context();
    let msg = build_handoff_message(&item, &context, "task_42", "/tmp/snap.json");
    assert_eq!(msg.source_role(), "system");
    assert!(msg.source_display().is_none());
    assert_eq!(msg.target_str(), "lead");
    assert_eq!(msg.task_id.as_deref(), Some("task_42"));
    assert!(msg.message.contains("Crash on launch"));
    assert!(msg.message.contains("1001"));
    let attachments = msg.attachments.unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].file_path, "/tmp/snap.json");
    assert_eq!(attachments[0].file_name, "task_42.json");
    assert!(!attachments[0].is_image);
}

#[test]
fn handoff_message_includes_repair_workflow_instructions() {
    let item = sample_item();
    let context = sample_context();
    let msg = build_handoff_message(&item, &context, "task_42", "/tmp/snap.json");
    assert!(msg.message.contains("repair plan"));
    assert!(msg.message.contains("plan \u{2192} execute \u{2192} review \u{2192} CM flow"));
    assert!(msg.message.contains("Feishu Project"));
}

#[test]
fn link_sets_linked_task_id_in_store_and_view() {
    let mut state = DaemonState::new();
    let item = sample_item();
    state.feishu_project_store.upsert(item.clone());
    state.feishu_issue_view = vec![item];
    let task = state.create_and_select_task("/ws", "[bug] Crash on launch");
    let tid = task.task_id.clone();
    if let Some(it) = state
        .feishu_project_store
        .find_by_work_item_id_mut("1001")
    {
        it.linked_task_id = Some(tid.clone());
    }
    if let Some(vi) = state.feishu_issue_view.iter_mut()
        .find(|i| i.work_item_id == "1001")
    {
        vi.linked_task_id = Some(tid.clone());
    }
    let item = state
        .feishu_project_store
        .find_by_work_item_id("1001")
        .unwrap();
    assert_eq!(item.linked_task_id.as_deref(), Some(tid.as_str()));
    assert_eq!(state.feishu_issue_view[0].linked_task_id.as_deref(), Some(tid.as_str()));
}

#[test]
fn relink_same_item_selects_existing_task() {
    let mut state = DaemonState::new();
    state.feishu_project_store.upsert(sample_item());
    let task = state.create_and_select_task("/ws", "[bug] Crash");
    let task_id = task.task_id.clone();
    if let Some(item) = state
        .feishu_project_store
        .find_by_work_item_id_mut("1001")
    {
        item.linked_task_id = Some(task_id.clone());
    }
    // Create another task to change active
    let _other = state.create_and_select_task("/ws", "Other");
    assert_ne!(state.active_task_id.as_deref(), Some(task_id.as_str()));
    // select_task should reactivate the linked task
    state.select_task(&task_id).unwrap();
    assert_eq!(state.active_task_id.as_deref(), Some(task_id.as_str()));
}

#[test]
fn handoff_description_from_plain_string() {
    let item = sample_item();
    let context = json!({
        "work_item_id": "1001",
        "name": "Crash on launch",
        "description": "App crashes on cold start",
    });
    let msg = build_handoff_message(&item, &context, "t1", "/tmp/s.json");
    assert!(msg.message.contains("App crashes on cold start"));
    assert!(!msg.message.contains("(no description)"));
}

#[test]
fn handoff_description_from_rich_text_object() {
    let item = sample_item();
    let context = json!({
        "work_item_id": "1001",
        "name": "Crash on launch",
        "description": {
            "doc_text": "Steps to reproduce:\n1. Open app\n2. Crash",
            "doc_type": 12,
            "content": [{"type": "paragraph"}]
        },
    });
    let msg = build_handoff_message(&item, &context, "t1", "/tmp/s.json");
    assert!(msg.message.contains("Steps to reproduce:"));
    assert!(!msg.message.contains("(no description)"));
}

#[test]
fn find_linked_bug_locates_item_by_task_id() {
    let mut state = DaemonState::new();
    let mut item = sample_item();
    let task = state.create_and_select_task("/ws", "[bug] Crash");
    item.linked_task_id = Some(task.task_id.clone());
    state.feishu_project_store.upsert(item);

    let found = find_linked_bug(&state.feishu_project_store, &task.task_id);
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.work_item_id, "1001");
    assert_eq!(found.project_key, "proj");
}

#[test]
fn find_linked_bug_returns_none_when_no_match() {
    let state = DaemonState::new();
    assert!(find_linked_bug(&state.feishu_project_store, "nonexistent").is_none());
}

#[test]
fn parse_transition_id_finds_processing_state() {
    let response = json!([
        {"id": 24640610, "state_name": "待处理", "state_key": "OPEN"},
        {"id": 24640619, "state_name": "处理中", "state_key": "IN PROGRESS"},
        {"id": 24640620, "state_name": "已完成", "state_key": "DONE"},
    ]);
    let tid = parse_transition_id(&response, "处理中");
    assert_eq!(tid, Some("24640619".to_string()));
}

#[test]
fn parse_transition_id_returns_none_when_missing() {
    let response = json!([
        {"id": 24640610, "state_name": "待处理", "state_key": "OPEN"},
    ]);
    assert!(parse_transition_id(&response, "处理中").is_none());
}

#[test]
fn parse_transition_id_handles_wrapped_data() {
    // Some responses wrap in {"data": [...]}
    let response = json!({"data": [
        {"id": 24640619, "state_name": "处理中", "state_key": "IN PROGRESS"},
    ]});
    let tid = parse_transition_id(&response, "处理中");
    assert_eq!(tid, Some("24640619".to_string()));
}

#[test]
fn build_transitable_states_args_uses_work_item_type() {
    let item = sample_item();
    let args = build_transitable_states_args(&item, "uk_123");
    // Feishu API requires "work_item_type", not "work_item_type_key"
    assert!(args.get("work_item_type").is_some(), "must use work_item_type");
    assert!(args.get("work_item_type_key").is_none(), "must NOT use work_item_type_key");
    assert_eq!(args["work_item_type"], "bug");
    assert_eq!(args["project_key"], "proj");
    assert_eq!(args["work_item_id"], "1001");
    assert_eq!(args["user_key"], "uk_123");
}

#[test]
fn handoff_description_fallback_when_missing() {
    let item = sample_item();
    let context = json!({
        "work_item_id": "1001",
        "name": "Crash on launch",
    });
    let msg = build_handoff_message(&item, &context, "t1", "/tmp/s.json");
    assert!(msg.message.contains("(no description)"));
}
