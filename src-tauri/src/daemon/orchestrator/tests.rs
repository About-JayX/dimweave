use crate::daemon::orchestrator::task_flow;
use crate::daemon::task_graph::types::*;
use crate::daemon::task_graph::TaskGraphStore;
use crate::daemon::types::{BridgeMessage, MessageSource, MessageTarget, MessageStatus};

fn make_source(role: &str) -> MessageSource {
    match role {
        "user" => MessageSource::User,
        "system" => MessageSource::System,
        _ => MessageSource::Agent {
            agent_id: "test".into(),
            role: role.into(),
            provider: Provider::Claude,
            display_source: None,
        },
    }
}

fn make_target(role: &str) -> MessageTarget {
    match role {
        "user" => MessageTarget::User,
        _ => MessageTarget::Role { role: role.into() },
    }
}

fn msg(from: &str, to: &str, status: MessageStatus) -> BridgeMessage {
    BridgeMessage {
        id: format!("{from}_to_{to}"),
        source: make_source(from),
        target: make_target(to),
        reply_target: None,
        message: "test".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        attachments: None,
    }
}

#[test]
fn coder_done_moves_task_into_reviewing() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Implementing);

    let released = task_flow::process_message(&mut store, &tid, &msg("coder", "lead", MessageStatus::Done));

    assert!(released.is_empty());
    let task = store.get_task(&tid).unwrap();
    assert_eq!(task.status, TaskStatus::Reviewing);
}

#[test]
fn lead_done_to_user_marks_task_done() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Reviewing);

    let released = task_flow::process_message(&mut store, &tid, &msg("lead", "user", MessageStatus::Done));

    assert!(released.is_empty());
    let task = store.get_task(&tid).unwrap();
    assert_eq!(task.status, TaskStatus::Done);
}

#[test]
fn unrelated_message_does_not_change_task_state() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Implementing);

    let _ = task_flow::process_message(&mut store, &tid, &msg("user", "coder", MessageStatus::Done));

    assert_eq!(store.get_task(&tid).unwrap().status, TaskStatus::Implementing);
}

#[test]
fn lead_to_coder_moves_task_into_implementing() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    // Task starts as Draft — lead→coder should move it to Implementing

    let released = task_flow::process_message(&mut store, &tid, &msg("lead", "coder", MessageStatus::InProgress));

    assert!(released.is_empty());
    let task = store.get_task(&tid).unwrap();
    assert_eq!(task.status, TaskStatus::Implementing);
}

#[test]
fn lead_to_coder_does_not_regress_from_implementing() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.update_task_status(&tid, TaskStatus::Implementing);

    // Second lead→coder message should keep Implementing, not re-transition
    let released = task_flow::process_message(&mut store, &tid, &msg("lead", "coder", MessageStatus::InProgress));

    assert!(released.is_empty());
    assert_eq!(store.get_task(&tid).unwrap().status, TaskStatus::Implementing);
}

#[test]
fn auto_target_returns_lead_during_review() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    store.update_task_status(&task.task_id, TaskStatus::Reviewing);
    let target = task_flow::preferred_auto_target(store.get_task(&task.task_id).unwrap());
    assert_eq!(target.as_deref(), Some("lead"));
}

#[test]
fn auto_target_returns_coder_during_implementing() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    store.update_task_status(&task.task_id, TaskStatus::Implementing);
    let target = task_flow::preferred_auto_target(store.get_task(&task.task_id).unwrap());
    assert_eq!(target.as_deref(), Some("coder"));
}
