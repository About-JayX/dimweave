use crate::daemon::state::DaemonState;
use crate::daemon::types::{Attachment, BridgeMessage};
use crate::feishu_project::types::{FeishuProjectInboxItem, IngressSource};

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

#[test]
fn new_task_binds_linked_task_id() {
    let mut state = DaemonState::new();
    state.feishu_project_store.upsert(sample_item());
    let task = state.create_and_select_task("/ws", "[bug] Crash on launch");
    if let Some(item) = state
        .feishu_project_store
        .find_by_work_item_id_mut("1001")
    {
        item.linked_task_id = Some(task.task_id.clone());
    }
    let item = state
        .feishu_project_store
        .find_by_work_item_id("1001")
        .unwrap();
    assert_eq!(item.linked_task_id.as_deref(), Some(task.task_id.as_str()));
}

#[test]
fn relink_reuses_existing_task() {
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
    let _other = state.create_and_select_task("/ws", "Other");
    assert_ne!(state.active_task_id.as_deref(), Some(task_id.as_str()));
    state.select_task(&task_id).unwrap();
    assert_eq!(state.active_task_id.as_deref(), Some(task_id.as_str()));
}

#[test]
fn handoff_message_structure() {
    let item = sample_item();
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let msg = BridgeMessage {
        id: format!("fp_handoff_task_42_{now}"),
        from: "system".into(),
        display_source: Some("feishu_project".into()),
        to: "lead".into(),
        content: format!(
            "Feishu Project repair task created.\n\n\
             **[{}]** {}\n\
             Type: `{}` | Source: {}\n\
             Task: `task_42`\n\n\
             Start by writing a repair plan from the attached snapshot, then follow \
             the plan \u{2192} execute \u{2192} review \u{2192} CM flow.",
            item.work_item_id, item.title, item.work_item_type_key, item.source_url,
        ),
        timestamp: now,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some("task_42".into()),
        session_id: None,
        sender_agent_id: None,
        attachments: Some(vec![Attachment {
            file_path: "/tmp/snap.json".into(),
            file_name: "task_42.json".into(),
            is_image: false,
            media_type: Some("application/json".into()),
        }]),
    };
    assert_eq!(msg.source_role(), "system");
    assert_eq!(msg.source_display(), Some("feishu_project"));
    assert_eq!(msg.target_str(), "lead");
    assert!(msg.content.contains("repair plan"));
    assert!(msg.content.contains("plan → execute → review → CM flow"));
    assert!(msg.content.contains("Feishu Project"));
    assert!(msg.content.contains("Crash on launch"));
    let att = msg.attachments.as_ref().unwrap();
    assert_eq!(att.len(), 1);
    assert!(!att[0].is_image);
}

#[test]
fn stale_linked_task_id_allows_new_task_creation() {
    let mut state = DaemonState::new();
    let mut item = sample_item();
    item.linked_task_id = Some("deleted_task_999".into());
    state.feishu_project_store.upsert(item);
    // select_task should fail for the stale id
    assert!(state.select_task("deleted_task_999").is_err());
    // A new task can still be created — the stale link does not block
    let task = state.create_and_select_task("/ws", "[bug] Crash on launch");
    assert!(state.active_task_id.as_deref() == Some(task.task_id.as_str()));
    // Rebind the linked_task_id to the new task
    if let Some(it) = state
        .feishu_project_store
        .find_by_work_item_id_mut("1001")
    {
        it.linked_task_id = Some(task.task_id.clone());
    }
    let it = state
        .feishu_project_store
        .find_by_work_item_id("1001")
        .unwrap();
    assert_eq!(it.linked_task_id.as_deref(), Some(task.task_id.as_str()));
}

#[test]
fn snapshot_round_trip() {
    let item = sample_item();
    let json = serde_json::to_string_pretty(&item).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["workItemId"], "1001");
    assert_eq!(parsed["title"], "Crash on launch");
    assert_eq!(parsed["projectKey"], "proj");
    assert_eq!(parsed["workItemTypeKey"], "bug");
}
